//! This module implements the API access to the Sentry API as well
//! as some other APIs we interact with.  In particular it can talk
//! to the GitHub API to figure out if there are new releases of the
//! sentry-cli tool.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::{create_dir_all, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fmt};

use anyhow::{Context, Result};
use backoff::backoff::Backoff;
use brotli2::write::BrotliEncoder;
use chrono::{DateTime, Duration, FixedOffset, Utc};
use clap::ArgMatches;
use flate2::write::GzEncoder;
use if_chain::if_chain;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use parking_lot::{Mutex, RwLock};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use regex::{Captures, Regex};
use sentry::protocol::{Exception, Values};
use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};
use sha1_smol::Digest;
use symbolic::common::DebugId;
use symbolic::debuginfo::ObjectKind;
use url::Url;
use uuid::Uuid;

use crate::config::{Auth, Config};
use crate::constants::{ARCH, EXT, PLATFORM, RELEASE_REGISTRY_LATEST_URL, VERSION};
use crate::utils::android::AndroidManifest;
use crate::utils::file_upload::UploadContext;
use crate::utils::http::{self, is_absolute_url, parse_link_header};
use crate::utils::progress::ProgressBar;
use crate::utils::retry::{get_default_backoff, DurationAsMilliseconds};
use crate::utils::sourcemaps::get_sourcemap_reference_from_headers;
use crate::utils::ui::{capitalize_string, make_byte_progress_bar};
use crate::utils::xcode::InfoPlist;

// Based on https://docs.rs/percent-encoding/1.0.1/src/percent_encoding/lib.rs.html#104
// WHATWG Spec: https://url.spec.whatwg.org/#percent-encoded-bytes
// RFC3986 Reserved Characters: https://www.rfc-editor.org/rfc/rfc3986#section-2.2
const QUERY_ENCODE_SET: AsciiSet = CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'+');
const PATH_SEGMENT_ENCODE_SET: AsciiSet = QUERY_ENCODE_SET
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'/');

/// Wrapper that escapes arguments for URL path segments.
pub struct PathArg<A: fmt::Display>(A);

/// Wrapper that escapes arguments for URL query segments.
pub struct QueryArg<A: fmt::Display>(A);

struct CurlConnectionManager;

impl r2d2::ManageConnection for CurlConnectionManager {
    type Connection = curl::easy::Easy;
    type Error = curl::Error;

    fn connect(&self) -> Result<curl::easy::Easy, curl::Error> {
        Ok(curl::easy::Easy::new())
    }

    fn is_valid(&self, _conn: &mut curl::easy::Easy) -> Result<(), curl::Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut curl::easy::Easy) -> bool {
        false
    }
}

lazy_static! {
    static ref API: Mutex<Option<Arc<Api>>> = Mutex::new(None);
}

#[derive(Debug, Clone)]
pub struct Link {
    results: bool,
    cursor: String,
}

#[derive(Debug, Default, Clone)]
pub struct Pagination {
    next: Option<Link>,
}

impl Pagination {
    pub fn into_next_cursor(self) -> Option<String> {
        self.next
            .and_then(|x| if x.results { Some(x.cursor) } else { None })
    }
}

impl FromStr for Pagination {
    type Err = ();

    fn from_str(s: &str) -> Result<Pagination, ()> {
        let mut rv = Pagination::default();
        for item in parse_link_header(s) {
            let target = match item.get("rel") {
                Some(&"next") => &mut rv.next,
                _ => continue,
            };

            *target = Some(Link {
                results: item.get("results") == Some(&"true"),
                cursor: (*item.get("cursor").unwrap_or(&"")).to_string(),
            });
        }

        Ok(rv)
    }
}

impl<A: fmt::Display> fmt::Display for QueryArg<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        utf8_percent_encode(&format!("{}", self.0), &QUERY_ENCODE_SET).fmt(f)
    }
}

impl<A: fmt::Display> fmt::Display for PathArg<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // if we put values into the path we need to url encode them.  However
        // special care needs to be taken for any slash character or path
        // segments that would end up as ".." or "." for security reasons.
        // Since we cannot handle slashes there we just replace them with the
        // unicode replacement character as a quick workaround.  This will
        // typically result in 404s from the server.
        let mut val = format!("{}", self.0).replace('/', "\u{fffd}");
        if val == ".." || val == "." {
            val = "\u{fffd}".into();
        }
        utf8_percent_encode(&val, &PATH_SEGMENT_ENCODE_SET).fmt(f)
    }
}

#[derive(Clone)]
pub enum ProgressBarMode {
    Disabled,
    Request,
    Response,
    Both,
    Shared((Arc<ProgressBar>, u64, usize, Arc<RwLock<Vec<u64>>>)),
}

impl ProgressBarMode {
    /// Returns if progress bars are generally enabled.
    pub fn active(&self) -> bool {
        !matches!(*self, ProgressBarMode::Disabled)
    }

    /// Returns whether a progress bar should be displayed during upload.
    pub fn request(&self) -> bool {
        matches!(*self, ProgressBarMode::Request | ProgressBarMode::Both)
    }

    /// Returns whether a progress bar should be displayed during download.
    pub fn response(&self) -> bool {
        matches!(*self, ProgressBarMode::Response | ProgressBarMode::Both)
    }
}

/// Helper for the API access.
pub struct Api {
    config: Arc<Config>,
    pool: r2d2::Pool<CurlConnectionManager>,
}

#[derive(Debug, thiserror::Error)]
pub struct SentryError {
    status: u32,
    detail: Option<String>,
    extra: Option<serde_json::Value>,
}

impl fmt::Display for SentryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = self.detail.as_deref().unwrap_or("");
        write!(
            f,
            "sentry reported an error: {} (http status: {})",
            if detail.is_empty() {
                match self.status {
                    400 => "bad request",
                    401 => "unauthorized",
                    404 => "not found",
                    500 => "internal server error",
                    502 => "bad gateway",
                    504 => "gateway timeout",
                    _ => "unknown error",
                }
            } else {
                detail
            },
            self.status
        )?;
        if let Some(ref extra) = self.extra {
            write!(f, "\n  {extra:?}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("project was renamed to '{0}'\nPlease use this slug in your .sentryclirc file, sentry.properties file or in the CLI --project parameter")]
pub struct ProjectRenamedError(String);

/// Represents API errors.
#[derive(Copy, Clone, Eq, PartialEq, Debug, thiserror::Error)]
pub enum ApiErrorKind {
    #[error("could not serialize value as JSON")]
    CannotSerializeAsJson,
    #[error("could not parse JSON response")]
    BadJson,
    #[error("not a JSON response")]
    NotJson,
    #[error("request failed because API URL was incorrectly formatted")]
    BadApiUrl,
    #[error("organization not found")]
    OrganizationNotFound,
    #[error("resource not found")]
    ResourceNotFound,
    #[error("project not found")]
    ProjectNotFound,
    #[error("release not found")]
    ReleaseNotFound,
    #[error("chunk upload endpoint not supported by sentry server")]
    ChunkUploadNotSupported,
    #[error("API request failed")]
    RequestFailed,
    #[error("could not compress data")]
    CompressionFailed,
}

#[derive(Debug, thiserror::Error)]
pub struct ApiError {
    inner: ApiErrorKind,
    #[source]
    source: Option<anyhow::Error>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl ApiError {
    pub fn with_source<E: Into<anyhow::Error>>(kind: ApiErrorKind, source: E) -> ApiError {
        ApiError {
            inner: kind,
            source: Some(source.into()),
        }
    }

    pub fn kind(&self) -> ApiErrorKind {
        self.inner
    }

    fn set_source<E: Into<anyhow::Error>>(mut self, source: E) -> ApiError {
        self.source = Some(source.into());
        self
    }
}

impl From<ApiErrorKind> for ApiError {
    fn from(kind: ApiErrorKind) -> ApiError {
        ApiError {
            inner: kind,
            source: None,
        }
    }
}

impl From<curl::Error> for ApiError {
    fn from(err: curl::Error) -> ApiError {
        ApiError::from(ApiErrorKind::RequestFailed).set_source(err)
    }
}

impl From<curl::FormError> for ApiError {
    fn from(err: curl::FormError) -> ApiError {
        ApiError::from(ApiErrorKind::RequestFailed).set_source(err)
    }
}

/// Shortcut alias for results of this module.
pub type ApiResult<T> = Result<T, ApiError>;

/// Represents an HTTP method that is used by the API.
#[derive(Eq, PartialEq, Debug)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Method::Get => write!(f, "GET"),
            Method::Head => write!(f, "HEAD"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
        }
    }
}

/// Represents an API request.  This can be customized before
/// sending but only sent once.
pub struct ApiRequest {
    url: String,
    handle: r2d2::PooledConnection<CurlConnectionManager>,
    headers: curl::easy::List,
    is_authenticated: bool,
    body: Option<Vec<u8>>,
    progress_bar_mode: ProgressBarMode,
    max_retries: u32,
    retry_on_statuses: &'static [u32],
}

/// Represents an API response.
#[derive(Clone, Debug)]
pub struct ApiResponse {
    url: String,
    status: u32,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl Api {
    /// Returns the current api for the thread.
    ///
    /// Threads other than the main thread must call `Api::reset` when
    /// shutting down to prevent `process::exit` from hanging afterwards.
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

    // Low Level Methods

    /// Create a new `ApiRequest` for the given HTTP method and URL.  If the
    /// URL is just a path then it's relative to the configured API host
    /// and authentication is automatically enabled.
    pub fn request(&self, method: Method, url: &str) -> ApiResult<ApiRequest> {
        let mut handle = self.pool.get().unwrap();
        handle.reset();
        if !self.config.allow_keepalive() {
            handle.forbid_reuse(true).ok();
        }
        let mut ssl_opts = curl::easy::SslOpt::new();
        if self.config.disable_ssl_revocation_check() {
            ssl_opts.no_revoke(true);
        }
        handle.ssl_options(&ssl_opts)?;
        let (url, auth) = if is_absolute_url(url) {
            (Cow::Borrowed(url), None)
        } else {
            (
                Cow::Owned(match self.config.get_api_endpoint(url) {
                    Ok(rv) => rv,
                    Err(err) => return Err(ApiError::with_source(ApiErrorKind::BadApiUrl, err)),
                }),
                self.config.get_auth(),
            )
        };

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

        // This toggles gzipping, useful for uploading large files
        handle.transfer_encoding(self.config.allow_transfer_encoding())?;

        let env = self.config.get_pipeline_env();
        let headers = self.config.get_headers();

        ApiRequest::create(handle, &method, &url, auth, env, headers)
    }

    /// Convenience method that performs a `GET` request.
    pub fn get(&self, path: &str) -> ApiResult<ApiResponse> {
        self.request(Method::Get, path)?.send()
    }

    /// Convenience method that performs a `DELETE` request.
    pub fn delete(&self, path: &str) -> ApiResult<ApiResponse> {
        self.request(Method::Delete, path)?.send()
    }

    /// Convenience method that performs a `POST` request with JSON data.
    pub fn post<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.request(Method::Post, path)?
            .with_json_body(body)?
            .send()
    }

    /// Convenience method that performs a `PUT` request with JSON data.
    pub fn put<S: Serialize>(&self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.request(Method::Put, path)?
            .with_json_body(body)?
            .send()
    }

    /// Convenience method that downloads a file into the given file object.
    pub fn download(&self, url: &str, dst: &mut File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, url)?
            .follow_location(true)?
            .send_into(dst)
    }

    /// Convenience method that downloads a file into the given file object
    /// and show a progress bar
    pub fn download_with_progress(&self, url: &str, dst: &mut File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, url)?
            .follow_location(true)?
            .progress_bar_mode(ProgressBarMode::Response)?
            .send_into(dst)
    }

    /// Convenience method that waits for a few seconds until a resource
    /// becomes available.
    pub fn wait_until_available(&self, url: &str, duration: Duration) -> ApiResult<bool> {
        let started = Utc::now();
        loop {
            match self.request(Method::Get, url)?.send() {
                Ok(_) => return Ok(true),
                Err(err) => {
                    if err.kind() != ApiErrorKind::RequestFailed {
                        return Err(err);
                    }
                }
            }
            std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());
            if Utc::now() - duration > started {
                return Ok(false);
            }
        }
    }

    // High Level Methods

    /// Performs an API request to verify the authentication status of the
    /// current token.
    pub fn get_auth_info(&self) -> ApiResult<AuthInfo> {
        self.get("/")?.convert()
    }

    /// Lists release files for the given `release`, filtered by a set of checksums.
    /// When empty checksums list is provided, fetches all possible artifacts.
    pub fn list_release_files_by_checksum(
        &self,
        org: &str,
        project: Option<&str>,
        release: &str,
        checksums: &[String],
    ) -> ApiResult<Vec<Artifact>> {
        let mut rv = vec![];
        let mut cursor = "".to_string();
        loop {
            let mut path = if let Some(project) = project {
                format!(
                    "/projects/{}/{}/releases/{}/files/?cursor={}",
                    PathArg(org),
                    PathArg(project),
                    PathArg(release),
                    QueryArg(&cursor),
                )
            } else {
                format!(
                    "/organizations/{}/releases/{}/files/?cursor={}",
                    PathArg(org),
                    PathArg(release),
                    QueryArg(&cursor),
                )
            };

            for checksum in checksums.iter() {
                path.push_str(&format!("&checksum={}", QueryArg(checksum)));
            }

            let resp = self.get(&path)?;
            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ReleaseNotFound.into());
                } else {
                    break;
                }
            }

            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Artifact>>()?.into_iter());
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    /// Lists all the release files for the given `release`.
    pub fn list_release_files(
        &self,
        org: &str,
        project: Option<&str>,
        release: &str,
    ) -> ApiResult<Vec<Artifact>> {
        self.list_release_files_by_checksum(org, project, release, &[])
    }

    /// Get a single release file and store it inside provided descriptor.
    pub fn get_release_file(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
        file_id: &str,
        file_desc: &mut File,
    ) -> Result<(), ApiError> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/files/{}/?download=1",
                PathArg(org),
                PathArg(project),
                PathArg(version),
                PathArg(file_id)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/{}/?download=1",
                PathArg(org),
                PathArg(version),
                PathArg(file_id)
            )
        };

        let resp = self.download(&path, file_desc)?;
        if resp.status() == 404 {
            resp.convert_rnf(ApiErrorKind::ResourceNotFound)
        } else {
            Ok(())
        }
    }

    /// Get a single release file metadata.
    pub fn get_release_file_metadata(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
        file_id: &str,
    ) -> ApiResult<Option<Artifact>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/files/{}/",
                PathArg(org),
                PathArg(project),
                PathArg(version),
                PathArg(file_id)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/{}/",
                PathArg(org),
                PathArg(version),
                PathArg(file_id)
            )
        };

        let resp = self.get(&path)?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    /// Deletes a single release file.  Returns `true` if the file was
    /// deleted or `false` otherwise.
    pub fn delete_release_file(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
        file_id: &str,
    ) -> ApiResult<bool> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/files/{}/",
                PathArg(org),
                PathArg(project),
                PathArg(version),
                PathArg(file_id)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/{}/",
                PathArg(org),
                PathArg(version),
                PathArg(file_id)
            )
        };

        let resp = self.delete(&path)?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.into_result().map(|_| true)
        }
    }

    /// Deletes all release files.  Returns `true` if files were
    /// deleted or `false` otherwise.
    pub fn delete_release_files(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
    ) -> ApiResult<bool> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/files/source-maps/?name={}",
                PathArg(org),
                PathArg(project),
                PathArg(version)
            )
        } else {
            format!(
                "/organizations/{}/files/source-maps/?name={}",
                PathArg(org),
                PathArg(version)
            )
        };

        let resp = self.delete(&path)?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.into_result().map(|_| true)
        }
    }

    /// Uploads a new release file.  The file is loaded directly from the file
    /// system and uploaded as `name`.
    pub fn upload_release_file(
        &self,
        context: &UploadContext,
        contents: &[u8],
        name: &str,
        headers: Option<&[(String, String)]>,
        progress_bar_mode: ProgressBarMode,
    ) -> ApiResult<Option<Artifact>> {
        let path = if let Some(project) = context.project {
            format!(
                "/projects/{}/{}/releases/{}/files/",
                PathArg(context.org),
                PathArg(project),
                PathArg(context.release)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/",
                PathArg(context.org),
                PathArg(context.release)
            )
        };
        let mut form = curl::easy::Form::new();

        let filename = Path::new(name)
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("unknown.bin");
        form.part("file")
            .buffer(filename, contents.to_vec())
            .add()?;
        form.part("name").contents(name.as_bytes()).add()?;
        if let Some(dist) = context.dist {
            form.part("dist").contents(dist.as_bytes()).add()?;
        }

        if let Some(headers) = headers {
            for (key, value) in headers {
                form.part("header")
                    .contents(format!("{key}:{value}").as_bytes())
                    .add()?;
            }
        }

        let resp = self
            .request(Method::Post, &path)?
            .with_form_data(form)?
            .with_retry(
                self.config.get_max_retry_count().unwrap(),
                &[
                    http::HTTP_STATUS_502_BAD_GATEWAY,
                    http::HTTP_STATUS_503_SERVICE_UNAVAILABLE,
                    http::HTTP_STATUS_504_GATEWAY_TIMEOUT,
                ],
            )?
            .progress_bar_mode(progress_bar_mode)?
            .send()?;
        if resp.status() == 409 {
            Ok(None)
        } else {
            resp.convert_rnf(ApiErrorKind::ReleaseNotFound)
        }
    }

    /// Creates a new release.
    pub fn new_release(&self, org: &str, release: &NewRelease) -> ApiResult<ReleaseInfo> {
        // for single project releases use the legacy endpoint that is project bound.
        // This means we can support both old and new servers.
        if release.projects.len() == 1 {
            let path = format!(
                "/projects/{}/{}/releases/",
                PathArg(org),
                PathArg(&release.projects[0])
            );
            self.post(&path, release)?
                .convert_rnf(ApiErrorKind::ProjectNotFound)
        } else {
            let path = format!("/organizations/{}/releases/", PathArg(org));
            self.post(&path, release)?
                .convert_rnf(ApiErrorKind::OrganizationNotFound)
        }
    }

    /// Updates a release.
    pub fn update_release(
        &self,
        org: &str,
        version: &str,
        release: &UpdatedRelease,
    ) -> ApiResult<ReleaseInfo> {
        if_chain! {
            if let Some(ref projects) = release.projects;
            if projects.len() == 1;
            then {
                let path = format!("/projects/{}/{}/releases/{}/",
                    PathArg(org),
                    PathArg(&projects[0]),
                    PathArg(version)
                );
                self.put(&path, release)?.convert_rnf(ApiErrorKind::ReleaseNotFound)
            } else {
                if release.version.is_some() {
                    let path = format!("/organizations/{}/releases/",
                                    PathArg(org));
                    return self.post(&path, release)?.convert_rnf(ApiErrorKind::ReleaseNotFound)
                }

                let path = format!("/organizations/{}/releases/{}/",
                                PathArg(org),
                                PathArg(version));
                self.put(&path, release)?.convert_rnf(ApiErrorKind::ReleaseNotFound)
            }
        }
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
    /// or `false` if not.  The project is needed to support the old deletion
    /// API.
    pub fn delete_release(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
    ) -> ApiResult<bool> {
        let resp = if let Some(project) = project {
            self.delete(&format!(
                "/projects/{}/{}/releases/{}/",
                PathArg(org),
                PathArg(project),
                PathArg(version)
            ))?
        } else {
            self.delete(&format!(
                "/organizations/{}/releases/{}/",
                PathArg(org),
                PathArg(version)
            ))?
        };
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.into_result().map(|_| true)
        }
    }

    /// Looks up a release and returns it.  If it does not exist `None`
    /// will be returned.
    pub fn get_release(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
    ) -> ApiResult<Option<ReleaseInfo>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/",
                PathArg(org),
                PathArg(project),
                PathArg(version)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/",
                PathArg(org),
                PathArg(version)
            )
        };
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
        project: Option<&str>,
        version: &str,
    ) -> ApiResult<Option<Vec<ReleaseCommit>>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/commits/",
                PathArg(org),
                PathArg(project),
                PathArg(version)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/commits/",
                PathArg(org),
                PathArg(version)
            )
        };
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
    pub fn create_deploy(&self, org: &str, version: &str, deploy: &Deploy) -> ApiResult<Deploy> {
        let path = format!(
            "/organizations/{}/releases/{}/deploys/",
            PathArg(org),
            PathArg(version)
        );

        self.post(&path, deploy)?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Lists all deploys for a release
    pub fn list_deploys(&self, org: &str, version: &str) -> ApiResult<Vec<Deploy>> {
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
                "/projects/{}/{}/issues/?{}",
                PathArg(org),
                PathArg(project),
                qs
            ),
            changes,
        )?
        .into_result()
        .map(|_| true)
    }

    /// Finds the latest release for sentry-cli on GitHub.
    pub fn get_latest_sentrycli_release(&self) -> ApiResult<Option<SentryCliRelease>> {
        let resp = self.get(RELEASE_REGISTRY_LATEST_URL)?;

        // Prefer universal binary on macOS
        let arch = match PLATFORM {
            "darwin" => "universal",
            _ => ARCH,
        };

        let ref_name = format!("sentry-cli-{}-{}{}", capitalize_string(PLATFORM), arch, EXT);
        info!("Looking for file named: {}", ref_name);

        if resp.status() == 200 {
            let info: RegistryRelease = resp.convert()?;
            for (filename, download_url) in info.file_urls {
                info!("Found asset {}", filename);
                if filename == ref_name {
                    return Ok(Some(SentryCliRelease {
                        version: info.version,
                        download_url,
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

    /// Given a list of checksums for DIFs, this returns a list of those
    /// that do not exist for the project yet.
    pub fn find_missing_dif_checksums<I>(
        &self,
        org: &str,
        project: &str,
        checksums: I,
    ) -> ApiResult<HashSet<Digest>>
    where
        I: IntoIterator<Item = Digest>,
    {
        let mut url = format!(
            "/projects/{}/{}/files/dsyms/unknown/?",
            PathArg(org),
            PathArg(project)
        );
        for (idx, checksum) in checksums.into_iter().enumerate() {
            if idx > 0 {
                url.push('&');
            }
            url.push_str("checksums=");
            url.push_str(&checksum.to_string());
        }

        let state: MissingChecksumsResponse = self.get(&url)?.convert()?;
        Ok(state.missing)
    }

    /// Uploads a ZIP archive containing DIFs from the given path.
    pub fn upload_dif_archive(
        &self,
        org: &str,
        project: &str,
        file: &Path,
    ) -> ApiResult<Vec<DebugInfoFile>> {
        let path = format!(
            "/projects/{}/{}/files/dsyms/",
            PathArg(org),
            PathArg(project)
        );
        let mut form = curl::easy::Form::new();
        form.part("file").file(file).add()?;
        self.request(Method::Post, &path)?
            .with_form_data(form)?
            .progress_bar_mode(ProgressBarMode::Request)?
            .send()?
            .convert()
    }

    /// Get the server configuration for chunked file uploads.
    pub fn get_chunk_upload_options(&self, org: &str) -> ApiResult<Option<ChunkUploadOptions>> {
        let url = format!("/organizations/{}/chunk-upload/", PathArg(org));
        match self
            .get(&url)?
            .convert_rnf(ApiErrorKind::ChunkUploadNotSupported)
        {
            Ok(options) => Ok(Some(options)),
            Err(error) => {
                if error.kind() == ApiErrorKind::ChunkUploadNotSupported {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
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
            .with_retry(
                self.config.get_max_retry_count().unwrap(),
                &[
                    http::HTTP_STATUS_502_BAD_GATEWAY,
                    http::HTTP_STATUS_503_SERVICE_UNAVAILABLE,
                    http::HTTP_STATUS_504_GATEWAY_TIMEOUT,
                ],
            )?
            .send()?
            .convert_rnf(ApiErrorKind::ProjectNotFound)
    }

    pub fn assemble_artifacts(
        &self,
        org: &str,
        release: &str,
        checksum: Digest,
        chunks: &[Digest],
    ) -> ApiResult<AssembleArtifactsResponse> {
        let url = format!(
            "/organizations/{}/releases/{}/assemble/",
            PathArg(org),
            PathArg(release)
        );

        self.request(Method::Post, &url)?
            .with_json_body(&ChunkedArtifactRequest { checksum, chunks })?
            .with_retry(
                self.config.get_max_retry_count().unwrap(),
                &[
                    http::HTTP_STATUS_502_BAD_GATEWAY,
                    http::HTTP_STATUS_503_SERVICE_UNAVAILABLE,
                    http::HTTP_STATUS_504_GATEWAY_TIMEOUT,
                ],
            )?
            .send()?
            .convert_rnf(ApiErrorKind::ReleaseNotFound)
    }

    /// Compresses a file with the given compression.
    fn compress(data: &[u8], compression: ChunkCompression) -> Result<Vec<u8>, io::Error> {
        Ok(match compression {
            ChunkCompression::Brotli => {
                let mut encoder = BrotliEncoder::new(Vec::new(), 6);
                encoder.write_all(data)?;
                encoder.finish()?
            }

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
        let stringified_chunks: Vec<_> = chunks
            .into_iter()
            .map(T::as_ref)
            .map(|&(checksum, data)| (checksum.to_string(), data))
            .collect();

        let mut form = curl::easy::Form::new();
        for (ref checksum, data) in stringified_chunks {
            let name = compression.field_name();
            let buffer = Api::compress(data, compression)
                .map_err(|err| ApiError::with_source(ApiErrorKind::CompressionFailed, err))?;
            form.part(name).buffer(&checksum, buffer).add()?
        }

        let request = self
            .request(Method::Post, url)?
            .with_form_data(form)?
            .with_retry(
                self.config.get_max_retry_count().unwrap(),
                &[
                    http::HTTP_STATUS_502_BAD_GATEWAY,
                    http::HTTP_STATUS_503_SERVICE_UNAVAILABLE,
                    http::HTTP_STATUS_504_GATEWAY_TIMEOUT,
                ],
            )?
            .progress_bar_mode(progress_bar_mode)?;

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

    /// Associate apple debug symbols with a build
    pub fn associate_apple_dsyms(
        &self,
        org: &str,
        project: &str,
        info_plist: &InfoPlist,
        checksums: Vec<String>,
    ) -> ApiResult<Option<AssociateDsymsResponse>> {
        self.associate_dsyms(
            org,
            project,
            &AssociateDsyms {
                platform: "apple".to_string(),
                checksums,
                name: info_plist.name().to_string(),
                app_id: info_plist.bundle_id().to_string(),
                version: info_plist.version().to_string(),
                build: Some(info_plist.build().to_string()),
            },
        )
    }

    /// Associate proguard mappings with an android app
    pub fn associate_android_proguard_mappings(
        &self,
        org: &str,
        project: &str,
        manifest: &AndroidManifest,
        checksums: Vec<String>,
    ) -> ApiResult<Option<AssociateDsymsResponse>> {
        self.associate_dsyms(
            org,
            project,
            &AssociateDsyms {
                platform: "android".to_string(),
                checksums,
                name: manifest.name(),
                app_id: manifest.package().to_string(),
                version: manifest.version_name().to_string(),
                build: Some(manifest.version_code().to_string()),
            },
        )
    }

    /// Associate arbitrary debug symbols with a build
    pub fn associate_dsyms(
        &self,
        org: &str,
        project: &str,
        data: &AssociateDsyms,
    ) -> ApiResult<Option<AssociateDsymsResponse>> {
        // in case we have no checksums to send up the server does not actually
        // let us associate anything.  This generally makes sense but means that
        // from the client side we need to deal with this separately.  In this
        // case we just pretend we did a request that did nothing.
        if data.checksums.is_empty() {
            return Ok(Some(AssociateDsymsResponse {
                associated_dsyms: vec![],
            }));
        }

        let path = format!(
            "/projects/{}/{}/files/dsyms/associate/",
            PathArg(org),
            PathArg(project)
        );
        let resp = self
            .request(Method::Post, &path)?
            .with_json_body(data)?
            .send()?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    /// Triggers reprocessing for a project
    pub fn trigger_reprocessing(&self, org: &str, project: &str) -> ApiResult<bool> {
        let path = format!(
            "/projects/{}/{}/reprocessing/",
            PathArg(org),
            PathArg(project)
        );
        let resp = self
            .request(Method::Post, &path)?
            .with_header("Content-Length", "0")?
            .send()?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.into_result().map(|_| true)
        }
    }

    /// List all organizations associated with the authenticated token
    pub fn list_organizations(&self) -> ApiResult<Vec<Organization>> {
        let mut rv = vec![];
        let mut cursor = "".to_string();
        loop {
            let resp = self.get(&format!("/organizations/?cursor={}", QueryArg(&cursor)))?;
            if resp.status() == 404 || (resp.status() == 400 && !cursor.is_empty()) {
                if rv.is_empty() {
                    return Err(ApiErrorKind::ResourceNotFound.into());
                } else {
                    break;
                }
            }
            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<Organization>>()?.into_iter());
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    /// List all monitors associated with an organization
    pub fn list_organization_monitors(&self, org: &str) -> ApiResult<Vec<Monitor>> {
        let mut rv = vec![];
        let mut cursor = "".to_string();
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
            rv.extend(resp.convert::<Vec<Monitor>>()?.into_iter());
            if let Some(next) = pagination.into_next_cursor() {
                cursor = next;
            } else {
                break;
            }
        }
        Ok(rv)
    }

    /// Create a new checkin for a monitor
    pub fn create_monitor_checkin(
        &self,
        monitor: &Uuid,
        checkin: &CreateMonitorCheckIn,
    ) -> ApiResult<MonitorCheckIn> {
        let path = &format!("/monitors/{}/checkins/", PathArg(monitor),);
        let resp = self.post(path, checkin)?;
        if resp.status() == 404 {
            return Err(ApiErrorKind::ResourceNotFound.into());
        }
        resp.convert()
    }

    /// Update a checkin for a monitor
    pub fn update_monitor_checkin(
        &self,
        monitor: &Uuid,
        checkin_id: &Uuid,
        checkin: &UpdateMonitorCheckIn,
    ) -> ApiResult<MonitorCheckIn> {
        let path = &format!(
            "/monitors/{}/checkins/{}/",
            PathArg(monitor),
            PathArg(checkin_id),
        );
        let resp = self.put(path, checkin)?;
        if resp.status() == 404 {
            return Err(ApiErrorKind::ResourceNotFound.into());
        }
        resp.convert()
    }

    /// List all projects associated with an organization
    pub fn list_organization_projects(&self, org: &str) -> ApiResult<Vec<Project>> {
        let mut rv = vec![];
        let mut cursor = "".to_string();
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
            rv.extend(resp.convert::<Vec<Project>>()?.into_iter());
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
        let mut cursor = "".to_string();
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
                    return Err(ApiErrorKind::OrganizationNotFound.into());
                } else {
                    break;
                }
            }

            let pagination = resp.pagination();
            rv.extend(resp.convert::<Vec<ProcessedEvent>>()?.into_iter());

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
        let mut cursor = "".to_string();
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
                rv.extend(resp.convert::<Vec<Repo>>()?.into_iter());
                if let Some(next) = pagination.into_next_cursor() {
                    cursor = next;
                } else {
                    break;
                }
            }
        }
        Ok(rv)
    }

    /// Looks up an event, which was already processed by Sentry and returns it.
    /// If it does not exist `None` will be returned.
    pub fn get_event(
        &self,
        org: &str,
        project: Option<&str>,
        event_id: &str,
    ) -> ApiResult<Option<ProcessedEvent>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/events/{}/json/",
                PathArg(org),
                PathArg(project),
                PathArg(event_id)
            )
        } else {
            format!(
                "/organizations/{}/events/{}/json/",
                PathArg(org),
                PathArg(event_id)
            )
        };

        let resp = self.get(&path)?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
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
            handle.progress_function(move |a, b, c, d| {
                let (down_len, down_pos, up_len, up_pos) = (a as u64, b as u64, c as u64, d as u64);
                let mut pb = pb_progress.borrow_mut();
                if up_len > 0 && progress_bar_mode.request() {
                    if up_pos < up_len {
                        if pb.is_none() {
                            *pb = Some(make_byte_progress_bar(up_len));
                        }
                        pb.as_ref().unwrap().set_position(up_pos);
                    } else if pb.is_some() {
                        pb.take().unwrap().finish_and_clear();
                    }
                }
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

    if pb.borrow().is_some() {
        pb.borrow().as_ref().unwrap().finish_and_clear();
    }

    Ok((handle.response_code()?, headers))
}

/// Iterator over response headers
#[allow(dead_code)]
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
        debug!("request {} {}", method, url);

        let mut headers = curl::easy::List::new();
        headers.append("Expect:").ok();

        if let Some(global_headers) = global_headers {
            for header in global_headers {
                headers.append(&header).ok();
            }
        }

        match pipeline_env {
            Some(env) => {
                debug!("pipeline: {}", env);
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
            Method::Head => {
                handle.get(true)?;
                handle.custom_request("HEAD")?;
                handle.nobody(true)?;
            }
            Method::Post => handle.custom_request("POST")?,
            Method::Put => handle.custom_request("PUT")?,
            Method::Delete => handle.custom_request("DELETE")?,
        }

        handle.url(url)?;

        let request = ApiRequest {
            url: url.to_owned(),
            handle,
            headers,
            is_authenticated: false,
            body: None,
            progress_bar_mode: ProgressBarMode::Disabled,
            max_retries: 0,
            retry_on_statuses: &[],
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
            Auth::Key(ref key) => {
                self.handle.username(key)?;
                debug!("using key based authentication");
                Ok(self)
            }
            Auth::Token(ref token) => {
                debug!("using token authentication");
                self.with_header("Authorization", &format!("Bearer {token}"))
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

    /// attaches some form data to the request.
    pub fn with_form_data(mut self, form: curl::easy::Form) -> ApiResult<Self> {
        debug!("sending form data");
        self.handle.httppost(form)?;
        self.body = None;
        Ok(self)
    }

    /// enables or disables redirects.  The default is off.
    pub fn follow_location(mut self, val: bool) -> ApiResult<Self> {
        debug!("follow redirects: {}", val);
        self.handle.follow_location(val)?;
        Ok(self)
    }

    /// enables a progress bar.
    pub fn progress_bar_mode(mut self, mode: ProgressBarMode) -> ApiResult<Self> {
        self.progress_bar_mode = mode;
        Ok(self)
    }

    pub fn with_retry(
        mut self,
        max_retries: u32,
        retry_on_statuses: &'static [u32],
    ) -> ApiResult<Self> {
        self.max_retries = max_retries;
        self.retry_on_statuses = retry_on_statuses;
        Ok(self)
    }

    /// Get a copy of the header list
    fn get_headers(&self) -> curl::easy::List {
        let mut result = curl::easy::List::new();
        for header_bytes in self.headers.iter() {
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
        let url = self.url.clone();
        let (status, headers) =
            send_req(&mut self.handle, out, body, self.progress_bar_mode.clone())?;
        debug!("response status: {}", status);
        Ok(ApiResponse {
            url,
            status,
            headers,
            body: None,
        })
    }

    /// Sends the request and reads the response body into the response object.
    pub fn send(mut self) -> ApiResult<ApiResponse> {
        let mut backoff = get_default_backoff();
        let mut retry_number = 0;

        loop {
            let mut out = vec![];
            debug!(
                "retry number {}, max retries: {}",
                retry_number, self.max_retries,
            );

            let mut rv = self.send_into(&mut out)?;
            if retry_number >= self.max_retries || !self.retry_on_statuses.contains(&rv.status) {
                rv.body = Some(out);
                return Ok(rv);
            }

            // Exponential backoff
            let backoff_timeout = backoff.next_backoff().unwrap();
            debug!(
                "retry number {}, retrying again in {} ms",
                retry_number,
                backoff_timeout.as_milliseconds()
            );
            std::thread::sleep(backoff_timeout);

            retry_number += 1;
        }
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
            debug!("body: {}", body);

            // Internal helper for making it easier to write integration tests.
            // Should not be used publicly, as it may be removed without prior warning.
            // Accepts a relative or absolute path to the directory where responses should be stored.
            if let Ok(dir) = env::var("SENTRY_DUMP_RESPONSES") {
                if let Err(err) = dump_response(dir, &self.url, body.into_owned()) {
                    debug!("Could not dump a response: {}", err);
                };
            }
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
    pub fn convert_rnf<T: DeserializeOwned>(self, res_err: ApiErrorKind) -> ApiResult<T> {
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
    #[allow(dead_code)]
    pub fn headers(&self) -> Headers<'_> {
        Headers {
            lines: &self.headers[..],
            idx: 0,
        }
    }

    /// Looks up the first matching header for a key.
    #[allow(dead_code)]
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
        static ref AUTH_RE: Regex = Regex::new(r"(?i)(authorization):\s*([\w]+)\s+(.*)").unwrap();
    }
    if let Ok(header) = std::str::from_utf8(data) {
        for line in header.lines() {
            if line.is_empty() {
                continue;
            }

            let replaced = AUTH_RE.replace_all(line, |caps: &Captures<'_>| {
                let info = if &caps[1].to_lowercase() == "basic" {
                    caps[3].split(':').next().unwrap().to_string()
                } else {
                    format!("{}***", &caps[3][..std::cmp::min(caps[3].len(), 8)])
                };
                format!("{}: {} {}", &caps[1], &caps[2], info)
            });
            debug!("{} {}", if is_response { ">" } else { "<" }, replaced);
        }
    }
}

fn dump_response(mut dir: String, url: &str, body: String) -> Result<()> {
    if dir.starts_with('~') {
        dir = format!(
            "{}{}",
            dirs::home_dir().unwrap_or_default().display(),
            dir.trim_start_matches('~')
        );
    }
    let filename = Url::parse(url)?.path().trim_matches('/').replace('/', "__");
    create_dir_all(&dir)?;
    let filepath = format!("{}/{}.json", &dir, filename);
    let mut file = File::create(&filepath)?;
    file.write_all(&body.into_bytes())?;
    debug!("Response dumped to: {}", &filepath);
    Ok(())
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
    pub id: String,
}

/// Provides the authentication information
#[derive(Deserialize, Debug)]
pub struct AuthInfo {
    pub auth: Option<AuthDetails>,
    pub user: Option<User>,
}

/// A release artifact
#[derive(Clone, Deserialize, Debug)]
pub struct Artifact {
    pub id: String,
    pub sha1: String,
    pub name: String,
    pub size: u64,
    pub dist: Option<String>,
    pub headers: HashMap<String, String>,
}

impl Artifact {
    pub fn get_header<'a>(&'a self, key: &str) -> Option<&'a str> {
        let ikey = key.to_lowercase();
        for (k, v) in &self.headers {
            if k.to_lowercase() == ikey {
                return Some(v.as_str());
            }
        }
        None
    }

    pub fn get_sourcemap_reference(&self) -> Option<&str> {
        get_sourcemap_reference_from_headers(self.headers.iter())
    }
}

/// Information for new releases
#[derive(Debug, Serialize, Default)]
pub struct NewRelease {
    pub version: String,
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
    #[serde(rename = "dateStarted", skip_serializing_if = "Option::is_none")]
    pub date_started: Option<DateTime<Utc>>,
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
struct GitHubAsset {
    browser_download_url: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegistryRelease {
    version: String,
    file_urls: HashMap<String, String>,
}

/// Information about sentry CLI releases
pub struct SentryCliRelease {
    pub version: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DebugInfoData {
    #[serde(default, rename = "type")]
    pub kind: Option<ObjectKind>,
    #[serde(default)]
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
    pub checksum: String,
    #[serde(default)]
    pub data: DebugInfoData,
}

impl DebugInfoFile {
    pub fn id(&self) -> DebugId {
        self.id.or(self.uuid).unwrap_or_default()
    }
}

#[derive(Debug, Serialize)]
pub struct AssociateDsyms {
    pub platform: String,
    pub checksums: Vec<String>,
    pub name: String,
    #[serde(rename = "appId")]
    pub app_id: String,
    pub version: String,
    pub build: Option<String>,
}

#[derive(Deserialize)]
struct MissingChecksumsResponse {
    missing: HashSet<Digest>,
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
        if matches.contains_id("all") {
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

#[derive(Deserialize)]
pub struct AssociateDsymsResponse {
    #[serde(rename = "associatedDsymFiles")]
    pub associated_dsyms: Vec<DebugInfoFile>,
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
    pub require_email_verification: bool,
    pub features: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Team {
    pub id: String,
    pub slug: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProjectSlugAndName {
    pub slug: String,
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
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorCheckinStatus {
    Unknown,
    Ok,
    InProgress,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct MonitorCheckIn {
    pub id: Uuid,
    pub status: MonitorCheckinStatus,
    pub duration: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateMonitorCheckIn {
    pub status: MonitorCheckinStatus,
}

#[derive(Debug, Serialize, Default)]
pub struct UpdateMonitorCheckIn {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<MonitorCheckinStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
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
    pub status: String,
    #[serde(rename = "dateCreated")]
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Deploy {
    #[serde(rename = "environment")]
    pub env: String,
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "dateStarted")]
    pub started: Option<DateTime<Utc>>,
    #[serde(rename = "dateFinished")]
    pub finished: Option<DateTime<Utc>>,
}

impl Deploy {
    /// Returns the name of this deploy, defaulting to `"unnamed"`.
    pub fn name(&self) -> &str {
        match self.name.as_deref() {
            Some("") | None => "unnamed",
            Some(name) => name,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ChunkHashAlgorithm {
    #[serde(rename = "sha1")]
    Sha1,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum ChunkCompression {
    /// No compression should be applied
    Uncompressed = 0,
    /// GZIP compression (including header)
    Gzip = 10,
    /// Brotli compression
    Brotli = 20,
}

impl ChunkCompression {
    fn field_name(self) -> &'static str {
        match self {
            ChunkCompression::Uncompressed => "file",
            ChunkCompression::Gzip => "file_gzip",
            ChunkCompression::Brotli => "file_brotli",
        }
    }
}

impl Default for ChunkCompression {
    fn default() -> Self {
        ChunkCompression::Uncompressed
    }
}

impl fmt::Display for ChunkCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ChunkCompression::Uncompressed => write!(f, "uncompressed"),
            ChunkCompression::Gzip => write!(f, "gzip"),
            ChunkCompression::Brotli => write!(f, "brotli"),
        }
    }
}

impl<'de> Deserialize<'de> for ChunkCompression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "gzip" => ChunkCompression::Gzip,
            "brotli" => ChunkCompression::Brotli,
            // We do not know this compression, so we assume no compression
            _ => ChunkCompression::Uncompressed,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkUploadCapability {
    /// Chunked upload of debug files
    DebugFiles,

    /// Chunked upload of release files
    ReleaseFiles,

    /// Upload of PDBs and debug id overrides
    Pdbs,

    /// Upload of Portable PDBs
    PortablePdbs,

    /// Uploads of source archives
    Sources,

    /// Upload of BCSymbolMap and PList auxiliary DIFs
    BcSymbolmap,

    /// Upload of il2cpp line mappings
    Il2Cpp,

    /// Any other unsupported capability (ignored)
    Unknown,
}

impl<'de> Deserialize<'de> for ChunkUploadCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "debug_files" => ChunkUploadCapability::DebugFiles,
            "release_files" => ChunkUploadCapability::ReleaseFiles,
            "pdbs" => ChunkUploadCapability::Pdbs,
            "portablepdbs" => ChunkUploadCapability::PortablePdbs,
            "sources" => ChunkUploadCapability::Sources,
            "bcsymbolmaps" => ChunkUploadCapability::BcSymbolmap,
            "il2cpp" => ChunkUploadCapability::Il2Cpp,
            _ => ChunkUploadCapability::Unknown,
        })
    }
}

fn default_chunk_upload_accept() -> Vec<ChunkUploadCapability> {
    vec![ChunkUploadCapability::DebugFiles]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkUploadOptions {
    pub url: String,
    #[serde(rename = "chunksPerRequest")]
    pub max_chunks: u64,
    #[serde(rename = "maxRequestSize")]
    pub max_size: u64,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default)]
    pub max_wait: u64,
    pub hash_algorithm: ChunkHashAlgorithm,
    pub chunk_size: u64,
    pub concurrency: u8,
    #[serde(default)]
    pub compression: Vec<ChunkCompression>,
    #[serde(default = "default_chunk_upload_accept")]
    pub accept: Vec<ChunkUploadCapability>,
}

impl ChunkUploadOptions {
    /// Returns whether the given capability is accepted by the chunk upload endpoint.
    pub fn supports(&self, capability: ChunkUploadCapability) -> bool {
        self.accept.contains(&capability)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ChunkedFileState {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "assembling")]
    Assembling,
    #[serde(rename = "ok")]
    Ok,
}

impl ChunkedFileState {
    pub fn is_finished(self) -> bool {
        self == ChunkedFileState::Error || self == ChunkedFileState::Ok
    }

    pub fn is_pending(self) -> bool {
        !self.is_finished()
    }

    pub fn is_err(self) -> bool {
        self == ChunkedFileState::Error || self == ChunkedFileState::NotFound
    }
}

#[derive(Debug, Serialize)]
pub struct ChunkedDifRequest<'a> {
    pub name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_id: Option<DebugId>,
    pub chunks: &'a [Digest],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkedDifResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
    pub dif: Option<DebugInfoFile>,
}

pub type AssembleDifsRequest<'a> = HashMap<Digest, ChunkedDifRequest<'a>>;
pub type AssembleDifsResponse = HashMap<Digest, ChunkedDifResponse>;

#[derive(Debug, Serialize)]
pub struct ChunkedArtifactRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleArtifactsResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
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
    pub project: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dist: Option<String>,
    #[serde(default, skip_serializing_if = "Values::is_empty")]
    pub exception: Values<Exception>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<ProcessedEventUser>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ProcessedEventTag>>,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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
