//! This module implements the API access to the Sentry API as well
//! as some other APIs we interact with.  In particular it can talk
//! to the GitHub API to figure out if there are new releases of the
//! sentry-cli tool.

pub mod envelopes_api;

mod connection_manager;
mod data_types;
mod encoding;
mod errors;
mod pagination;
mod serialization;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(any(target_os = "macos", not(feature = "managed")))]
use std::fs::File;
use std::io::{self, Read as _, Write};
use std::rc::Rc;
use std::sync::Arc;
use std::{fmt, thread};

use anyhow::{Context as _, Result};
use backon::BlockingRetryable as _;
#[cfg(target_os = "macos")]
use chrono::Duration;
use chrono::{DateTime, FixedOffset, Utc};
use clap::ArgMatches;
use flate2::write::GzEncoder;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use parking_lot::Mutex;
use regex::{Captures, Regex};
use secrecy::ExposeSecret as _;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha1_smol::Digest;
use symbolic::common::DebugId;
use symbolic::debuginfo::ObjectKind;
use uuid::Uuid;

use crate::api::errors::{ProjectRenamedError, RetryError};
use crate::config::{Auth, Config};
use crate::constants::{ARCH, EXT, PLATFORM, RELEASE_REGISTRY_LATEST_URL, VERSION};
use crate::utils::http::{self, is_absolute_url};
use crate::utils::non_empty::NonEmptySlice;
use crate::utils::progress::{ProgressBar, ProgressBarMode};
use crate::utils::retry::{get_default_backoff, DurationAsMilliseconds as _};
use crate::utils::ui::{capitalize_string, make_byte_progress_bar};

use self::pagination::Pagination;
use connection_manager::CurlConnectionManager;
use encoding::{PathArg, QueryArg};
use errors::{ApiError, ApiErrorKind, ApiResult, SentryError};

pub use self::data_types::*;

lazy_static! {
    static ref API: Mutex<Option<Arc<Api>>> = Mutex::new(None);
}

const RETRY_STATUS_CODES: &[u32] = &[
    http::HTTP_STATUS_502_BAD_GATEWAY,
    http::HTTP_STATUS_503_SERVICE_UNAVAILABLE,
    http::HTTP_STATUS_504_GATEWAY_TIMEOUT,
    http::HTTP_STATUS_507_INSUFFICIENT_STORAGE,
    http::HTTP_STATUS_524_CLOUDFLARE_TIMEOUT,
];

/// Helper for the API access.
/// Implements the low-level API access methods, and provides high-level implementations for interacting
/// with portions of the API that do not require authentication via an auth token.
pub struct Api {
    config: Arc<Config>,
    pool: r2d2::Pool<CurlConnectionManager>,
}

/// Wrapper for Api that ensures Auth is provided. AuthenticatedApi provides implementations of high-level
/// functions that make API requests requiring authentication via auth token.
pub struct AuthenticatedApi<'a> {
    api: &'a Api,
}

/// Represents an HTTP method that is used by the API.
#[derive(Eq, PartialEq, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
        }
    }
}

/// Represents an API request.  This can be customized before
/// sending but only sent once.
pub struct ApiRequest {
    handle: r2d2::PooledConnection<CurlConnectionManager>,
    headers: curl::easy::List,
    is_authenticated: bool,
    body: Option<Vec<u8>>,
    progress_bar_mode: ProgressBarMode,
}

/// Represents an API response.
#[derive(Clone, Debug)]
pub struct ApiResponse {
    status: u32,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl<'a> TryFrom<&'a Api> for AuthenticatedApi<'a> {
    type Error = ApiError;

    fn try_from(api: &'a Api) -> ApiResult<AuthenticatedApi<'a>> {
        match api.config.get_auth() {
            Some(_) => Ok(AuthenticatedApi { api }),
            None => Err(ApiErrorKind::AuthMissing.into()),
        }
    }
}

impl Api {
    /// Returns the current api for the thread.
    pub fn current() -> Arc<Api> {
        let mut api_opt = API.lock();
        if let Some(ref api) = *api_opt {
            api.clone()
        } else {
            let api = Arc::new(Api::with_config(Config::current()));
            *api_opt = Some(api.clone());
            api
        }
    }

    /// Similar to `new` but uses a specific config.
    pub fn with_config(config: Arc<Config>) -> Api {
        Api {
            config,
            #[expect(clippy::unwrap_used, reason = "legacy code")]
            pool: r2d2::Pool::builder()
                .max_size(16)
                .build(CurlConnectionManager)
                .unwrap(),
        }
    }

    /// Utility method that unbinds the current api.
    pub fn dispose_pool() {
        *API.lock() = None;
    }

    /// Creates an AuthenticatedApi referencing this Api instance if an auth token is available.
    /// If an auth token is not available, returns an error.
    pub fn authenticated(&self) -> ApiResult<AuthenticatedApi<'_>> {
        self.try_into()
    }

    // Low Level Methods

    /// Create a new `ApiRequest` for the given HTTP method and URL.  If the
    /// URL is just a path then it's relative to the configured API host
    /// and authentication is automatically enabled.
    fn request(
        &self,
        method: Method,
        url: &str,
        region_url: Option<&str>,
    ) -> ApiResult<ApiRequest> {
        let (url, auth) = self.resolve_base_url_and_auth(url, region_url)?;
        self.construct_api_request(method, &url, auth)
    }

    fn resolve_base_url_and_auth(
        &self,
        url: &str,
        region_url: Option<&str>,
    ) -> ApiResult<(String, Option<&Auth>)> {
        if is_absolute_url(url) && region_url.is_some() {
            return Err(ApiErrorKind::InvalidRegionRequest.into());
        }

        let (url, auth) = if is_absolute_url(url) {
            (Cow::Borrowed(url), None)
        } else {
            (
                Cow::Owned(match self.config.get_api_endpoint(url, region_url) {
                    Ok(rv) => rv,
                    Err(err) => return Err(ApiError::with_source(ApiErrorKind::BadApiUrl, err)),
                }),
                self.config.get_auth(),
            )
        };

        Ok((url.into_owned(), auth))
    }

    fn construct_api_request(
        &self,
        method: Method,
        url: &str,
        auth: Option<&Auth>,
    ) -> ApiResult<ApiRequest> {
        let mut handle = self
            .pool
            .get()
            .map_err(|e| ApiError::with_source(ApiErrorKind::RequestFailed, e))?;

        handle.reset();
        if !self.config.allow_keepalive() {
            handle.forbid_reuse(true).ok();
        }
        let mut ssl_opts = curl::easy::SslOpt::new();
        if self.config.disable_ssl_revocation_check() {
            ssl_opts.no_revoke(true);
        }
        handle.ssl_options(&ssl_opts)?;

        if let Some(proxy_url) = self.config.get_proxy_url() {
            handle.proxy(&proxy_url)?;
        }
        if let Some(proxy_username) = self.config.get_proxy_username() {
            handle.proxy_username(proxy_username)?;
        }
        if let Some(proxy_password) = self.config.get_proxy_password() {
            handle.proxy_password(proxy_password)?;
        }
        handle.ssl_verify_host(self.config.should_verify_ssl())?;
        handle.ssl_verify_peer(self.config.should_verify_ssl())?;

        let env = self.config.get_pipeline_env();
        let headers = self.config.get_headers();

        ApiRequest::create(handle, &method, url, auth, env, headers)
    }

    /// Convenience method that performs a `GET` request.
    fn get(&self, path: &str) -> ApiResult<ApiResponse> {
        self.request(Method::Get, path, None)?.send()
    }

    /// Convenience method that performs a `DELETE` request.
    fn delete(&self, path: &str) -> ApiResult<ApiResponse> {
        self.request(Method::Delete, path, None)?.send()
    }

    /// Convenience method that performs a `POST` request with JSON data.
    fn post<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.request(Method::Post, path, None)?
            .with_json_body(body)?
            .send()
    }

    /// Convenience method that performs a `PUT` request with JSON data.
    fn put<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.request(Method::Put, path, None)?
            .with_json_body(body)?
            .send()
    }

    /// Convenience method that downloads a file into the given file object.
    ///
    /// Currently only used on macOS, but we could make it available on other platforms
    /// if needed.
    #[cfg(target_os = "macos")]
    pub fn download(&self, url: &str, dst: &mut File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, url, None)?
            .follow_location(true)?
            .send_into(dst)
    }

    /// Convenience method that downloads a file into the given file object
    /// and show a progress bar
    #[cfg(not(feature = "managed"))]
    pub fn download_with_progress(&self, url: &str, dst: &mut File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, url, None)?
            .follow_location(true)?
            .progress_bar_mode(ProgressBarMode::Response)
            .send_into(dst)
    }

    /// Convenience method that waits for a few seconds until a resource
    /// becomes available. We only use this in the macOS binary.
    #[cfg(target_os = "macos")]
    pub fn wait_until_available(&self, url: &str, duration: Duration) -> ApiResult<bool> {
        let started = Utc::now();
        loop {
            match self.request(Method::Get, url, None)?.send() {
                Ok(_) => return Ok(true),
                Err(err) => {
                    if err.kind() != ApiErrorKind::RequestFailed {
                        return Err(err);
                    }
                }
            }
            std::thread::sleep(
                Duration::milliseconds(500)
                    .to_std()
                    .expect("500ms is valid, as it is non-negative"),
            );
            if Utc::now() - duration > started {
                return Ok(false);
            }
        }
    }

    // High Level Methods

    /// Finds the latest release for sentry-cli on GitHub.
    pub fn get_latest_sentrycli_release(&self) -> ApiResult<Option<SentryCliRelease>> {
        let resp = self.get(RELEASE_REGISTRY_LATEST_URL)?;

        // Prefer universal binary on macOS
        let arch = match PLATFORM {
            "darwin" => "universal",
            _ => ARCH,
        };

        let ref_name = format!("sentry-cli-{}-{arch}{EXT}", capitalize_string(PLATFORM));
        info!("Looking for file named: {ref_name}");

        if resp.status() == 200 {
            let info: RegistryRelease = resp.convert()?;
            for (filename, _download_url) in info.file_urls {
                info!("Found asset {filename}");
                if filename == ref_name {
                    return Ok(Some(SentryCliRelease {
                        version: info.version,
                        #[cfg(not(feature = "managed"))]
                        download_url: _download_url,
                    }));
                }
            }
            warn!("Unable to find release file");
            Ok(None)
        } else {
            info!("Release registry returned {}", resp.status());
            Ok(None)
        }
    }

    /// Compresses a file with the given compression.
    fn compress(data: &[u8], compression: ChunkCompression) -> Result<Vec<u8>, io::Error> {
        Ok(match compression {
            ChunkCompression::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Default::default());
                encoder.write_all(data)?;
                encoder.finish()?
            }

            ChunkCompression::Uncompressed => data.into(),
        })
    }

    /// Upload a batch of file chunks.
    pub fn upload_chunks<'data, I, T>(
        &self,
        url: &str,
        chunks: I,
        progress_bar_mode: ProgressBarMode,
        compression: ChunkCompression,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = &'data T>,
        T: AsRef<(Digest, &'data [u8])> + 'data,
    {
        // Curl stores a raw pointer to the stringified checksum internally. We first
        // transform all checksums to string and keep them in scope until the request
        // has completed. The original iterator is not needed anymore after this.
        let stringified_chunks = chunks
            .into_iter()
            .map(T::as_ref)
            .map(|&(checksum, data)| (checksum.to_string(), data));

        let mut form = curl::easy::Form::new();
        for (ref checksum, data) in stringified_chunks {
            let name = compression.field_name();
            let buffer = Api::compress(data, compression)
                .map_err(|err| ApiError::with_source(ApiErrorKind::CompressionFailed, err))?;
            form.part(name).buffer(&checksum, buffer).add()?
        }

        let request = self
            .request(Method::Post, url, None)?
            .with_form_data(form)?
            .progress_bar_mode(progress_bar_mode);

        // The request is performed to an absolute URL. Thus, `Self::request()` will
        // not add the authorization header, by default. Since the URL is guaranteed
        // to be a Sentry-compatible endpoint, we force the Authorization header at
        // this point.
        let request = match Config::current().get_auth() {
            // Make sure that we don't authenticate a request
            // that has been already authenticated
            Some(auth) if !request.is_authenticated => request.with_auth(auth)?,
            _ => request,
        };

        // Handle 301 or 302 requests as a missing project
        let resp = request.send()?;
        match resp.status() {
            301 | 302 => Err(ApiErrorKind::ProjectNotFound.into()),
            _ => {
                resp.into_result()?;
                Ok(())
            }
        }
    }
}

impl AuthenticatedApi<'_> {
    // Pass through low-level methods to API.

    /// Convenience method to call self.api.get.
    fn get(&self, path: &str) -> ApiResult<ApiResponse> {
        self.api.get(path)
    }

    /// Convenience method to call self.api.delete.
    fn delete(&self, path: &str) -> ApiResult<ApiResponse> {
        self.api.delete(path)
    }

    /// Convenience method to call self.api.post.
    fn post<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.api.post(path, body)
    }

    /// Convenience method to call self.api.put.
    fn put<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.api.put(path, body)
    }

    /// Convenience method to call self.api.request.
    fn request(&self, method: Method, url: &str) -> ApiResult<ApiRequest> {
        self.api.request(method, url, None)
    }

    // High-level method implementations

    /// Performs an API request to verify the authentication status of the
    /// current token.
    pub fn get_auth_info(&self) -> ApiResult<AuthInfo> {
        self.get("/")?.convert()
    }

    /// Creates a new release.
    pub fn new_release(&self, org: &str, release: &NewRelease) -> ApiResult<ReleaseInfo> {
        let path = format!("/organizations/{}/releases/", PathArg(org));
        self.post(&path, release)?
            .convert_rnf(ApiErrorKind::OrganizationNotFound)
    }

    /// Updates a release.
    pub fn update_release(
        &self,
        org: &str,
        version: &str,
        release: &UpdatedRelease,
    ) -> ApiResult<ReleaseInfo> {
        if release.version.is_some() {
            let path = format!("/organizations/{}/releases/", PathArg(org));
            return self
                .post(&path, release)?
                .convert_rnf(ApiErrorKind::ReleaseNotFound);
        }

        let path = format!(
            "/organizations/{}/releases/{}/",
            PathArg(org),
            PathArg(version)
        );
        self.put(&path, release)?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Sets release commits
    pub fn set_release_refs(
        &self,
        org: &str,
        version: &str,
        refs: Vec<Ref>,
    ) -> ApiResult<ReleaseInfo> {
        let update = UpdatedRelease {
            refs: Some(refs),
            ..Default::default()
        };
        let path = format!(
            "/organizations/{}/releases/{}/",
            PathArg(org),
            PathArg(version)
        );
        self.put(&path, &update)?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Deletes an already existing release.  Returns `true` if it was deleted
    /// or `false` if not.
    pub fn delete_release(&self, org: &str, version: &str) -> ApiResult<bool> {
        let resp = self.delete(&format!(
            "/organizations/{}/releases/{}/",
            PathArg(org),
            PathArg(version)
        ))?;

        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.into_result().map(|_| true)
        }
    }

    /// Looks up a release and returns it.  If it does not exist `None`
    /// will be returned.
    pub fn get_release(&self, org: &str, version: &str) -> ApiResult<Option<ReleaseInfo>> {
        let path = format!(
            "/organizations/{}/releases/{}/",
            PathArg(org),
            PathArg(version)
        );
        let resp = self.get(&path)?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    /// Returns a list of releases for a given project.  This is currently a
    /// capped list by what the server deems an acceptable default limit.
    pub fn list_releases(&self, org: &str, project: Option<&str>) -> ApiResult<Vec<ReleaseInfo>> {
        if let Some(project) = project {
            let path = format!("/projects/{}/{}/releases/", PathArg(org), PathArg(project));
            self.get(&path)?
                .convert_rnf::<Vec<ReleaseInfo>>(ApiErrorKind::ProjectNotFound)
        } else {
            let path = format!("/organizations/{}/releases/", PathArg(org));
            self.get(&path)?
                .convert_rnf::<Vec<ReleaseInfo>>(ApiErrorKind::OrganizationNotFound)
        }
    }

    /// Looks up a release commits and returns it.  If it does not exist `None`
    /// will be returned.
    pub fn get_release_commits(
        &self,
        org: &str,
        version: &str,
    ) -> ApiResult<Option<Vec<ReleaseCommit>>> {
        let path = format!(
            "/organizations/{}/releases/{}/commits/",
            PathArg(org),
            PathArg(version)
        );
        let resp = self.get(&path)?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    // Finds the most recent release with commits and returns it.
    // If it does not exist `None` will be returned.
    pub fn get_previous_release_with_commits(
        &self,
        org: &str,
        version: &str,
    ) -> ApiResult<OptionalReleaseInfo> {
        let path = format!(
            "/organizations/{}/releases/{}/previous-with-commits/",
            PathArg(org),
            PathArg(version)
        );
        let resp = self.get(&path)?;
        if resp.status() == 404 {
            Ok(OptionalReleaseInfo::None(NoneReleaseInfo {}))
        } else {
            resp.convert()
        }
    }

    /// Creates a new deploy for a release.
    pub fn create_deploy(
        &self,
        org: &str,
        version: &str,
        deploy: &Deploy,
    ) -> ApiResult<Deploy<'_>> {
        let path = format!(
            "/organizations/{}/releases/{}/deploys/",
            PathArg(org),
            PathArg(version)
        );

        self.post(&path, deploy)?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Lists all deploys for a release
    pub fn list_deploys(&self, org: &str, version: &str) -> ApiResult<Vec<Deploy<'_>>> {
        let path = format!(
            "/organizations/{}/releases/{}/deploys/",
            PathArg(org),
            PathArg(version)
        );
        self.get(&path)?.convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Updates a bunch of issues within a project that match a provided filter
    /// and performs `changes` changes.
    pub fn bulk_update_issue(
        &self,
        org: &str,
        project: &str,
        filter: &IssueFilter,
        changes: &IssueChanges,
    ) -> ApiResult<bool> {
        let qs = match filter.get_query_string() {
            None => {
                return Ok(false);
            }
            Some(qs) => qs,
        };
        self.put(
            &format!(
                "/projects/{}/{}/issues/?{qs}",
                PathArg(org),
                PathArg(project)
            ),
            changes,
        )?
        .into_result()
        .map(|_| true)
    }

    /// Get the server configuration for chunked file uploads.
    pub fn get_chunk_upload_options(&self, org: &str) -> ApiResult<ChunkServerOptions> {
        let url = format!("/organizations/{}/chunk-upload/", PathArg(org));
        self.get(&url)?
            .convert_rnf::<ChunkServerOptions>(ApiErrorKind::OrganizationNotFound)
    }

    /// Request DIF assembling and processing from chunks.
    pub fn assemble_difs(
        &self,
        org: &str,
        project: &str,
        request: &AssembleDifsRequest<'_>,
    ) -> ApiResult<AssembleDifsResponse> {
        let url = format!(
            "/projects/{}/{}/files/difs/assemble/",
            PathArg(org),
            PathArg(project)
        );

        self.request(Method::Post, &url)?
            .with_json_body(request)?
            .send()?
            .convert_rnf(ApiErrorKind::ProjectNotFound)
    }

    pub fn assemble_artifact_bundle(
        &self,
        org: &str,
        projects: NonEmptySlice<'_, String>,
        checksum: Digest,
        chunks: &[Digest],
        version: Option<&str>,
        dist: Option<&str>,
    ) -> ApiResult<AssembleArtifactsResponse> {
        let url = format!("/organizations/{}/artifactbundle/assemble/", PathArg(org));

        self.request(Method::Post, &url)?
            .with_json_body(&ChunkedArtifactRequest {
                checksum,
                chunks,
                projects: projects.into(),
                version,
                dist,
            })?
            .send()?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    pub fn assemble_build(
        &self,
        org: &str,
        project: &str,
        request: &ChunkedBuildRequest<'_>,
    ) -> ApiResult<AssembleBuildResponse> {
        let url = format!(
            "/projects/{}/{}/files/preprodartifacts/assemble/",
            PathArg(org),
            PathArg(project)
        );

        self.request(Method::Post, &url)?
            .with_json_body(&request)?
            .send()?
            .convert_rnf(ApiErrorKind::ProjectNotFound)
    }

    /// List all organizations associated with the authenticated token
    /// in the given `Region`. If no `Region` is provided, we assume
    /// we're issuing a request to a monolith deployment.
    pub fn list_organizations(&self, region: Option<&Region>) -> ApiResult<Vec<Organization>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        loop {
            let current_path = &format!("/organizations/?cursor={}", QueryArg(&cursor));
            let resp = if let Some(rg) = region {
                self.api
                    .request(Method::Get, current_path, Some(&rg.url))?
                    .send()?
            } else {
                self.get(current_path)?
            };

            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ResourceNotFound.into());
                } else {
                    break;
                }
            }
            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Organization>>()?);
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    pub fn list_available_regions(&self) -> ApiResult<Vec<Region>> {
        let resp = self.get("/users/me/regions/")?;
        if resp.status() == 404 {
            // This endpoint may not exist for self-hosted users, so
            // returning a default of [] seems appropriate.
            return Ok(vec![]);
        }

        if resp.status() == 400 {
            return Err(ApiErrorKind::ResourceNotFound.into());
        }

        let region_response = resp.convert::<RegionResponse>()?;
        Ok(region_response.regions)
    }

    /// List all monitors associated with an organization
    pub fn list_organization_monitors(&self, org: &str) -> ApiResult<Vec<Monitor>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        loop {
            let resp = self.get(&format!(
                "/organizations/{}/monitors/?cursor={}",
                PathArg(org),
                QueryArg(&cursor)
            ))?;
            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ResourceNotFound.into());
                } else {
                    break;
                }
            }
            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Monitor>>()?);
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    /// List all projects associated with an organization
    pub fn list_organization_projects(&self, org: &str) -> ApiResult<Vec<Project>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        loop {
            let resp = self.get(&format!(
                "/organizations/{}/projects/?cursor={}",
                PathArg(org),
                QueryArg(&cursor)
            ))?;
            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::OrganizationNotFound.into());
                } else {
                    break;
                }
            }
            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Project>>()?);
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    /// List all events associated with an organization and a project
    pub fn list_organization_project_events(
        &self,
        org: &str,
        project: &str,
        max_pages: usize,
    ) -> ApiResult<Vec<ProcessedEvent>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        let mut requests_no = 0;

        loop {
            requests_no += 1;

            let resp = self.get(&format!(
                "/projects/{}/{}/events/?cursor={}",
                PathArg(org),
                PathArg(project),
                QueryArg(&cursor)
            ))?;

            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ProjectNotFound.into());
                } else {
                    break;
                }
            }

            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<ProcessedEvent>>()?);

            if requests_no == max_pages {
                break;
            }

            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }

        Ok(rv)
    }

    /// Fetch organization events from the specified dataset
    pub fn fetch_organization_events(
        &self,
        org: &str,
        options: &FetchEventsOptions,
    ) -> ApiResult<Vec<LogEntry>> {
        let params = options.to_query_params();
        let url = format!(
            "/organizations/{}/events/?{}",
            PathArg(org),
            params.join("&")
        );

        let resp = self.get(&url)?;

        if resp.status() == 404 {
            return Err(ApiErrorKind::OrganizationNotFound.into());
        }

        let logs_response: LogsResponse = resp.convert()?;
        Ok(logs_response.data)
    }

    /// List all issues associated with an organization and a project
    pub fn list_organization_project_issues(
        &self,
        org: &str,
        project: &str,
        max_pages: usize,
        query: Option<String>,
    ) -> ApiResult<Vec<Issue>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        let mut requests_no = 0;

        let url = if let Some(query) = query {
            format!(
                "/projects/{}/{}/issues/?query={}&",
                PathArg(org),
                PathArg(project),
                QueryArg(&query),
            )
        } else {
            format!("/projects/{}/{}/issues/?", PathArg(org), PathArg(project),)
        };

        loop {
            requests_no += 1;

            let resp = self.get(&format!("{url}cursor={}", QueryArg(&cursor)))?;

            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ProjectNotFound.into());
                } else {
                    break;
                }
            }

            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Issue>>()?);

            if requests_no == max_pages {
                break;
            }

            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }

        Ok(rv)
    }

    /// List all repos associated with an organization
    pub fn list_organization_repos(&self, org: &str) -> ApiResult<Vec<Repo>> {
        let mut rv = vec![];
        let mut cursor = "".to_owned();
        loop {
            let path = format!(
                "/organizations/{}/repos/?cursor={}",
                PathArg(org),
                QueryArg(&cursor)
            );
            let resp = self.request(Method::Get, &path)?.send()?;
            if resp.status() == 404 {
                break;
            } else {
                let pagination = resp.pagination();
                rv.extend(resp.convert::<Vec<Repo>>()?);
                if let Some(next) = pagination.into_next_cursor() {
                    cursor = next;
                } else {
                    break;
                }
            }
        }
        Ok(rv)
    }

    /// Creates a preprod snapshot artifact for the given project.
    pub fn create_preprod_snapshot<S: Serialize>(
        &self,
        org: &str,
        project: &str,
        body: &S,
    ) -> ApiResult<ApiResponse> {
        let path = format!(
            "/projects/{}/{}/preprodartifacts/snapshots/",
            PathArg(org),
            PathArg(project)
        );
        self.post(&path, body)
    }

    /// Fetches upload options for snapshots.
    pub fn fetch_snapshots_upload_options(
        &self,
        org: &str,
        project: &str,
    ) -> ApiResult<SnapshotsUploadOptions> {
        let path = format!(
            "/projects/{}/{}/preprodartifacts/snapshots/upload-options/",
            PathArg(org),
            PathArg(project)
        );
        self.get(&path)?.convert()
    }
}

/// Available datasets for fetching organization events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dataset {
    /// Our logs dataset
    Logs,
}

impl Dataset {
    /// Returns the string representation of the dataset
    fn as_str(&self) -> &'static str {
        match self {
            Dataset::Logs => "logs",
        }
    }
}

impl fmt::Display for Dataset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Options for fetching organization events
pub struct FetchEventsOptions<'a> {
    /// Dataset to fetch events from
    pub dataset: Dataset,
    /// Fields to include in the response
    pub fields: &'a [&'a str],
    /// Project ID to filter events by
    pub project_id: Option<&'a str>,
    /// Cursor for pagination
    pub cursor: Option<&'a str>,
    /// Query string to filter events
    pub query: &'a str,
    /// Number of events per page
    pub per_page: usize,
    /// Time period for stats
    pub stats_period: &'a str,
    /// Sort order
    pub sort: &'a str,
}

impl FetchEventsOptions<'_> {
    /// Generate query parameters as a vector of strings
    pub fn to_query_params(&self) -> Vec<String> {
        let mut params = vec![format!("dataset={}", QueryArg(self.dataset.as_str()))];

        for field in self.fields {
            params.push(format!("field={}", QueryArg(field)));
        }

        if let Some(cursor) = self.cursor {
            params.push(format!("cursor={}", QueryArg(cursor)));
        }

        if let Some(project) = self.project_id {
            if !project.is_empty() {
                params.push(format!("project={}", QueryArg(project)));
            }
        }
        if !self.query.is_empty() {
            params.push(format!("query={}", QueryArg(self.query)));
        }
        params.push(format!("per_page={}", self.per_page));
        params.push(format!("statsPeriod={}", QueryArg(self.stats_period)));
        params.push(format!("sort={}", QueryArg(self.sort)));

        params
    }
}

fn send_req<W: Write>(
    handle: &mut curl::easy::Easy,
    out: &mut W,
    body: Option<&[u8]>,
    progress_bar_mode: ProgressBarMode,
) -> ApiResult<(u32, Vec<String>)> {
    match body {
        Some(mut body) => {
            handle.upload(true)?;
            handle.in_filesize(body.len() as u64)?;
            handle_req(handle, out, progress_bar_mode, &mut |buf| {
                body.read(buf).unwrap_or(0)
            })
        }
        None => handle_req(handle, out, progress_bar_mode, &mut |_| 0),
    }
}

fn handle_req<W: Write>(
    handle: &mut curl::easy::Easy,
    out: &mut W,
    progress_bar_mode: ProgressBarMode,
    read: &mut dyn FnMut(&mut [u8]) -> usize,
) -> ApiResult<(u32, Vec<String>)> {
    if progress_bar_mode.active() {
        handle.progress(true)?;
    }

    // enable verbose mode
    handle.verbose(true)?;

    let mut headers = Vec::new();
    let pb: Rc<RefCell<Option<ProgressBar>>> = Rc::new(RefCell::new(None));
    {
        let headers = &mut headers;
        let mut handle = handle.transfer();

        if let ProgressBarMode::Shared((pb_progress, len, idx, counts)) = progress_bar_mode {
            handle.progress_function(move |_, _, total, uploaded| {
                if uploaded > 0f64 && uploaded < total {
                    counts.write()[idx] = (uploaded / total * (len as f64)) as u64;
                    pb_progress.set_position(counts.read().iter().sum());
                }
                true
            })?;
        } else if progress_bar_mode.active() {
            let pb_progress = pb.clone();
            #[expect(clippy::unwrap_used, reason = "legacy code")]
            handle.progress_function(move |a, b, _, _| {
                let (down_len, down_pos) = (a as u64, b as u64);
                let mut pb = pb_progress.borrow_mut();
                if down_len > 0 && progress_bar_mode.response() {
                    if down_pos < down_len {
                        if pb.is_none() {
                            *pb = Some(make_byte_progress_bar(down_len));
                        }
                        pb.as_ref().unwrap().set_position(down_pos);
                    } else if pb.is_some() {
                        pb.take().unwrap().finish_and_clear();
                    }
                }
                true
            })?;
        }

        handle.read_function(move |buf| Ok(read(buf)))?;

        handle.write_function(move |data| {
            Ok(match out.write_all(data) {
                Ok(_) => data.len(),
                Err(_) => 0,
            })
        })?;

        handle.debug_function(move |info, data| match info {
            curl::easy::InfoType::HeaderIn => {
                log_headers(false, data);
            }
            curl::easy::InfoType::HeaderOut => {
                log_headers(true, data);
            }
            _ => {}
        })?;

        handle.header_function(move |data| {
            headers.push(String::from_utf8_lossy(data).into_owned());
            true
        })?;
        handle.perform()?;
    }

    if let Some(pb) = pb.borrow().as_ref() {
        pb.finish_and_clear();
    }

    Ok((handle.response_code()?, headers))
}

/// Iterator over response headers
pub struct Headers<'a> {
    lines: &'a [String],
    idx: usize,
}

impl<'a> Iterator for Headers<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        self.lines.get(self.idx).map(|line| {
            self.idx += 1;
            match line.find(':') {
                Some(i) => (&line[..i], line[i + 1..].trim()),
                None => (line[..].trim(), ""),
            }
        })
    }
}

impl ApiRequest {
    fn create(
        mut handle: r2d2::PooledConnection<CurlConnectionManager>,
        method: &Method,
        url: &str,
        auth: Option<&Auth>,
        pipeline_env: Option<String>,
        global_headers: Option<Vec<String>>,
    ) -> ApiResult<Self> {
        let mut headers = curl::easy::List::new();
        headers.append("Expect:").ok();

        if let Some(global_headers) = global_headers {
            for header in global_headers {
                headers.append(&header).ok();
            }
        }

        match pipeline_env {
            Some(env) => {
                debug!("pipeline: {env}");
                headers
                    .append(&format!("User-Agent: sentry-cli/{VERSION} {env}"))
                    .ok();
            }
            None => {
                headers
                    .append(&format!("User-Agent: sentry-cli/{VERSION}"))
                    .ok();
            }
        }

        match method {
            Method::Get => handle.get(true)?,
            Method::Post => handle.custom_request("POST")?,
            Method::Put => handle.custom_request("PUT")?,
            Method::Delete => handle.custom_request("DELETE")?,
        }

        handle.url(url)?;

        let request = ApiRequest {
            handle,
            headers,
            is_authenticated: false,
            body: None,
            progress_bar_mode: ProgressBarMode::Disabled,
        };

        let request = match auth {
            Some(auth) => ApiRequest::with_auth(request, auth)?,
            None => request,
        };

        Ok(request)
    }

    /// Explicitly overrides the Auth info.
    pub fn with_auth(mut self, auth: &Auth) -> ApiResult<Self> {
        self.is_authenticated = true;
        match *auth {
            Auth::Token(ref token) => {
                debug!("using token authentication");
                self.with_header(
                    "Authorization",
                    &format!("Bearer {}", token.raw().expose_secret()),
                )
            }
        }
    }

    /// adds a specific header to the request
    pub fn with_header(mut self, key: &str, value: &str) -> ApiResult<Self> {
        let value = value.trim().lines().next().unwrap_or("");
        self.headers.append(&format!("{key}: {value}"))?;
        Ok(self)
    }

    /// sets the JSON request body for the request.
    pub fn with_json_body<S: Serialize>(mut self, body: &S) -> ApiResult<Self> {
        let mut body_bytes: Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &body)
            .map_err(|err| ApiError::with_source(ApiErrorKind::CannotSerializeAsJson, err))?;
        debug!("json body: {}", String::from_utf8_lossy(&body_bytes));
        self.body = Some(body_bytes);
        self.headers.append("Content-Type: application/json")?;
        Ok(self)
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// attaches some form data to the request.
    pub fn with_form_data(mut self, form: curl::easy::Form) -> ApiResult<Self> {
        debug!("sending form data");
        self.handle.httppost(form)?;
        self.body = None;
        Ok(self)
    }

    /// enables or disables redirects.  The default is off.
    #[cfg(any(target_os = "macos", not(feature = "managed")))]
    pub fn follow_location(mut self, val: bool) -> ApiResult<Self> {
        debug!("follow redirects: {val}");
        self.handle.follow_location(val)?;
        Ok(self)
    }

    /// enables a progress bar.
    pub fn progress_bar_mode(mut self, mode: ProgressBarMode) -> Self {
        self.progress_bar_mode = mode;
        self
    }

    /// Get a copy of the header list
    fn get_headers(&self) -> curl::easy::List {
        let mut result = curl::easy::List::new();
        for header_bytes in self.headers.iter() {
            #[expect(clippy::unwrap_used, reason = "legacy code")]
            let header = String::from_utf8(header_bytes.to_vec()).unwrap();
            result.append(&header).ok();
        }
        result
    }

    /// Sends the request and writes response data into the given file
    /// instead of the response object's in memory buffer.
    pub fn send_into<W: Write>(&mut self, out: &mut W) -> ApiResult<ApiResponse> {
        let headers = self.get_headers();
        self.handle.http_headers(headers)?;
        let body = self.body.as_deref();
        let (status, headers) =
            send_req(&mut self.handle, out, body, self.progress_bar_mode.clone())?;
        Ok(ApiResponse {
            status,
            headers,
            body: None,
        })
    }

    /// Sends the request and reads the response body into the response object.
    pub fn send(mut self) -> ApiResult<ApiResponse> {
        let max_retries = Config::current().max_retries();

        let backoff = get_default_backoff().with_max_times(max_retries as usize);
        let retry_number = RefCell::new(0);

        let send_req = || {
            let mut out = vec![];
            let mut retry_number = retry_number.borrow_mut();

            debug!("retry number {retry_number}, max retries: {max_retries}");
            *retry_number += 1;

            let mut rv = self.send_into(&mut out)?;
            rv.body = Some(out);

            if RETRY_STATUS_CODES.contains(&rv.status) {
                anyhow::bail!(RetryError::new(rv));
            }

            Ok(rv)
        };

        send_req
            .retry(backoff)
            .sleep(thread::sleep)
            .when(|e| e.is::<RetryError>())
            .notify(|e, dur| {
                debug!(
                    "retry number {} failed due to {e:#}, retrying again in {} ms",
                    *retry_number.borrow() - 1,
                    dur.as_milliseconds()
                );
            })
            .call()
            .or_else(|err| match err.downcast::<RetryError>() {
                Ok(err) => Ok(err.into_body()),
                Err(err) => Err(ApiError::with_source(ApiErrorKind::RequestFailed, err)),
            })
    }
}

impl ApiResponse {
    /// Returns the status code of the response
    pub fn status(&self) -> u32 {
        self.status
    }

    /// Indicates that the request failed
    pub fn failed(&self) -> bool {
        self.status >= 400 && self.status <= 600
    }

    /// Indicates that the request succeeded
    pub fn ok(&self) -> bool {
        !self.failed()
    }

    /// Converts the API response into a result object.  This also converts
    /// non okay response codes into errors.
    pub fn into_result(self) -> ApiResult<Self> {
        if let Some(ref body) = self.body {
            let body = String::from_utf8_lossy(body);
            debug!("body: {body}");
        }
        if self.ok() {
            return Ok(self);
        }
        if let Ok(err) = self.deserialize::<ErrorInfo>() {
            Err(ApiError::with_source(
                ApiErrorKind::RequestFailed,
                SentryError {
                    status: self.status(),
                    detail: Some(match err {
                        ErrorInfo::Detail(val) => val,
                        ErrorInfo::Error(val) => val,
                    }),
                    extra: None,
                },
            ))
        } else if let Ok(value) = self.deserialize::<serde_json::Value>() {
            Err(ApiError::with_source(
                ApiErrorKind::RequestFailed,
                SentryError {
                    status: self.status(),
                    detail: Some("request failure".into()),
                    extra: Some(value),
                },
            ))
        } else {
            Err(ApiError::with_source(
                ApiErrorKind::RequestFailed,
                SentryError {
                    status: self.status(),
                    detail: None,
                    extra: None,
                },
            ))
        }
    }

    /// Deserializes the response body into the given type
    pub fn deserialize<T: DeserializeOwned>(&self) -> ApiResult<T> {
        if !self.is_json() {
            return Err(ApiErrorKind::NotJson.into());
        }
        serde_json::from_reader(match self.body {
            Some(ref body) => body,
            None => &b""[..],
        })
        .map_err(|err| ApiError::with_source(ApiErrorKind::BadJson, err))
    }

    /// Like `deserialize` but consumes the response and will convert
    /// failed requests into proper errors.
    pub fn convert<T: DeserializeOwned>(self) -> ApiResult<T> {
        self.into_result().and_then(|x| x.deserialize())
    }

    /// Like convert but produces resource not found errors.
    fn convert_rnf<T: DeserializeOwned>(self, res_err: ApiErrorKind) -> ApiResult<T> {
        match self.status() {
            301 | 302 if res_err == ApiErrorKind::ProjectNotFound => {
                #[derive(Deserialize, Debug)]
                struct ErrorDetail {
                    slug: String,
                }

                #[derive(Deserialize, Debug)]
                struct ErrorInfo {
                    detail: ErrorDetail,
                }

                match self.convert::<ErrorInfo>() {
                    Ok(info) => Err(ApiError::with_source(
                        res_err,
                        ProjectRenamedError(info.detail.slug),
                    )),
                    Err(_) => Err(res_err.into()),
                }
            }
            404 => Err(res_err.into()),
            _ => self.into_result().and_then(|x| x.deserialize()),
        }
    }

    /// Iterates over the headers.
    pub fn headers(&self) -> Headers<'_> {
        Headers {
            lines: &self.headers[..],
            idx: 0,
        }
    }

    /// Looks up the first matching header for a key.
    pub fn get_header(&self, key: &str) -> Option<&str> {
        for (header_key, header_value) in self.headers() {
            if header_key.eq_ignore_ascii_case(key) {
                return Some(header_value);
            }
        }
        None
    }

    /// Returns the pagination info
    pub fn pagination(&self) -> Pagination {
        self.get_header("link")
            .and_then(|x| x.parse().ok())
            .unwrap_or_default()
    }

    /// Returns true if the response is JSON.
    pub fn is_json(&self) -> bool {
        self.get_header("content-type")
            .and_then(|x| x.split(';').next())
            .unwrap_or("")
            == "application/json"
    }
}

fn log_headers(is_response: bool, data: &[u8]) {
    lazy_static! {
        static ref AUTH_RE: Regex =
            Regex::new(r"(?i)(authorization):\s*([\w]+)\s+(.*)").expect("regex is valid");
    }
    if let Ok(header) = std::str::from_utf8(data) {
        for line in header.lines() {
            if line.is_empty() {
                continue;
            }

            let replaced = AUTH_RE.replace_all(line, |caps: &Captures<'_>| {
                let info = if &caps[1].to_lowercase() == "basic" {
                    #[expect(clippy::unwrap_used, reason = "legacy code")]
                    caps[3].split(':').next().unwrap().to_owned()
                } else {
                    format!("{}***", &caps[3][..std::cmp::min(caps[3].len(), 8)])
                };
                format!("{}: {} {info}", &caps[1], &caps[2])
            });
            debug!("{} {replaced}", if is_response { ">" } else { "<" });
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ErrorInfo {
    Detail(String),
    Error(String),
}

/// Provides the auth details (access scopes)
#[derive(Deserialize, Debug)]
pub struct AuthDetails {
    pub scopes: Vec<String>,
}

/// Indicates which user signed in
#[derive(Deserialize, Debug)]
pub struct User {
    pub email: String,
    #[expect(dead_code)]
    pub id: String,
}

/// Provides the authentication information
#[derive(Deserialize, Debug)]
pub struct AuthInfo {
    pub auth: Option<AuthDetails>,
    pub user: Option<User>,
}

/// Information for new releases
#[derive(Debug, Serialize, Default)]
pub struct NewRelease {
    pub version: String,
    #[serde(serialize_with = "serialization::serialize_id_slug_list")]
    pub projects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "dateStarted", skip_serializing_if = "Option::is_none")]
    pub date_started: Option<DateTime<Utc>>,
    #[serde(rename = "dateReleased", skip_serializing_if = "Option::is_none")]
    pub date_released: Option<DateTime<Utc>>,
}

/// A head commit on a release
#[derive(Debug, Serialize, Default)]
pub struct Ref {
    #[serde(rename = "repository")]
    pub repo: String,
    #[serde(rename = "commit")]
    pub rev: String,
    #[serde(rename = "previousCommit")]
    pub prev_rev: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseStatus {
    Open,
    Archived,
}

/// Changes to a release
#[derive(Debug, Serialize, Default)]
pub struct UpdatedRelease {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "dateReleased", skip_serializing_if = "Option::is_none")]
    pub date_released: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs: Option<Vec<Ref>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<GitCommit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ReleaseStatus>,
}

/// Provides all release information from already existing releases
#[derive(Debug, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    #[expect(dead_code)]
    pub url: Option<String>,
    #[serde(rename = "dateCreated")]
    pub date_created: DateTime<Utc>,
    #[serde(default, rename = "dateReleased")]
    pub date_released: Option<DateTime<Utc>>,
    #[serde(default, rename = "lastEvent")]
    pub last_event: Option<DateTime<Utc>>,
    #[serde(default, rename = "newGroups")]
    pub new_groups: u64,
    #[serde(default)]
    pub projects: Vec<ProjectSlugAndName>,
    #[serde(
        default,
        rename = "lastCommit",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_commit: Option<ReleaseCommit>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OptionalReleaseInfo {
    None(NoneReleaseInfo),
    Some(ReleaseInfo),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoneReleaseInfo {}

#[derive(Debug, Deserialize)]
pub struct ReleaseCommit {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegistryRelease {
    version: String,
    file_urls: HashMap<String, String>,
}

/// Information about sentry CLI releases
pub struct SentryCliRelease {
    pub version: String,
    #[cfg(not(feature = "managed"))]
    pub download_url: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DebugInfoData {
    #[serde(default, rename = "type")]
    pub kind: Option<ObjectKind>,
    #[serde(default)]
    #[expect(dead_code)]
    pub features: Vec<String>,
}

/// Debug information files as processed and stored on the server.
/// Can be dSYMs, ELF debug infos, Breakpad symbols, etc...
#[derive(Debug, Deserialize)]
pub struct DebugInfoFile {
    #[serde(rename = "uuid")]
    uuid: Option<DebugId>,
    #[serde(rename = "debugId")]
    id: Option<DebugId>,
    #[serde(rename = "objectName")]
    pub object_name: String,
    #[serde(rename = "cpuName")]
    pub cpu_name: String,
    #[serde(rename = "sha1")]
    #[expect(dead_code)]
    pub checksum: String,
    #[serde(default)]
    pub data: DebugInfoData,
}

impl DebugInfoFile {
    pub fn id(&self) -> DebugId {
        self.id.or(self.uuid).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub last_seen: String,
    pub status: String,
    pub level: String,
}

/// Change information for issue bulk updates.
#[derive(Serialize, Default)]
pub struct IssueChanges {
    #[serde(rename = "status")]
    pub new_status: Option<String>,
    #[serde(rename = "snoozeDuration")]
    pub snooze_duration: Option<i64>,
}

/// Filters for issue bulk requests.
pub enum IssueFilter {
    /// Match no issues
    Empty,
    /// Match on all issues
    All,
    /// Match on the issues with the given IDs
    ExplicitIds(Vec<u64>),
    /// Match on issues with the given status
    Status(String),
}

impl IssueFilter {
    fn get_query_string(&self) -> Option<String> {
        let mut rv = vec![];
        match *self {
            IssueFilter::Empty => {
                return None;
            }
            IssueFilter::All => {}
            IssueFilter::ExplicitIds(ref ids) => {
                if ids.is_empty() {
                    return None;
                }
                for id in ids {
                    rv.push(format!("id={id}"));
                }
            }
            IssueFilter::Status(ref status) => {
                rv.push(format!("status={status}"));
            }
        }
        Some(rv.join("&"))
    }

    pub fn get_filter_from_matches(matches: &ArgMatches) -> Result<IssueFilter> {
        if matches.get_flag("all") {
            return Ok(IssueFilter::All);
        }
        if let Some(status) = matches.get_one::<String>("status") {
            return Ok(IssueFilter::Status(status.into()));
        }
        let mut ids = vec![];
        if let Some(values) = matches.get_many::<String>("id") {
            for value in values {
                ids.push(value.parse::<u64>().context("Invalid issue ID")?);
            }
        }

        if ids.is_empty() {
            Ok(IssueFilter::Empty)
        } else {
            Ok(IssueFilter::ExplicitIds(ids))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Organization {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(rename = "dateCreated")]
    pub date_created: DateTime<Utc>,
    #[serde(rename = "isEarlyAdopter")]
    pub is_early_adopter: bool,
    #[serde(rename = "require2FA")]
    pub require_2fa: bool,
    #[serde(rename = "requireEmailVerification")]
    #[expect(dead_code)]
    pub require_email_verification: bool,
    #[expect(dead_code)]
    pub features: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Team {
    #[expect(dead_code)]
    pub id: String,
    #[expect(dead_code)]
    pub slug: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProjectSlugAndName {
    pub slug: String,
    #[expect(dead_code)]
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub team: Option<Team>,
}

#[derive(Debug, Deserialize)]
pub struct Monitor {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct RepoProvider {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub provider: RepoProvider,
    #[expect(dead_code)]
    pub status: String,
    #[serde(rename = "dateCreated")]
    #[expect(dead_code)]
    pub date_created: DateTime<Utc>,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.provider.id, &self.id)?;
        if let Some(ref url) = self.url {
            write!(f, " ({url})")?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct PatchSet {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct GitCommit {
    pub patch_set: Vec<PatchSet>,
    pub repository: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub timestamp: DateTime<FixedOffset>,
    pub message: Option<String>,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProcessedEvent {
    #[serde(alias = "eventID")]
    pub event_id: Uuid,
    #[serde(default, alias = "dateCreated")]
    pub date_created: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[expect(dead_code)]
    pub project: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<ProcessedEventUser>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ProcessedEventTag>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProcessedEventUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

impl fmt::Display for ProcessedEventUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(id) = &self.id {
            write!(f, "ID: {id}")?;
        }

        if let Some(username) = &self.username {
            write!(f, "Username: {username}")?;
        }

        if let Some(email) = &self.email {
            write!(f, "Email: {email}")?;
        }

        if let Some(ip_address) = &self.ip_address {
            write!(f, "IP: {ip_address}")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProcessedEventTag {
    pub key: String,
    pub value: String,
}

impl fmt::Display for ProcessedEventTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", &self.key, &self.value)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Region {
    #[expect(dead_code)]
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegionResponse {
    pub regions: Vec<Region>,
}

/// Response structure for logs API
#[derive(Debug, Deserialize)]
struct LogsResponse {
    data: Vec<LogEntry>,
}

/// Log entry structure from the logs API
#[derive(Debug, Deserialize, Clone)]
pub struct LogEntry {
    #[serde(rename = "sentry.item_id")]
    pub item_id: String,
    pub trace: Option<String>,
    pub severity: Option<String>,
    pub timestamp: String,
    pub message: Option<String>,
}

/// Upload options returned by the snapshots upload-options endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotsUploadOptions {
    pub objectstore: ObjectstoreUploadOptions,
}

/// Objectstore configuration for file uploads.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectstoreUploadOptions {
    pub url: String,
    pub scopes: Vec<(String, String)>,
    pub expiration_policy: String,
}
