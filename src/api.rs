//! This module implements the API access to the Sentry API as well
//! as some other APIs we interact with.  In particular it can talk
//! to the GitHub API to figure out if there are new releases of the
//! sentry-cli tool.

use std::io;
use std::fs;
use std::str;
use std::io::{Read, Write};
use std::fmt;
use std::error;
use std::thread;
use std::sync::Arc;
use std::cell::{RefCell, RefMut};
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::borrow::Cow;
use std::rc::Rc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use curl;
use chrono::{DateTime, Duration, Utc};
use indicatif::ProgressBar;
use regex::{Captures, Regex};

use utils;
use utils::xcode::InfoPlist;
use event::Event;
use config::{Auth, Config, Dsn};
use constants::{ARCH, EXT, PLATFORM, VERSION};

/// Wrapper that escapes arguments for URL path segments.
pub struct PathArg<A: fmt::Display>(A);

impl<A: fmt::Display> fmt::Display for PathArg<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        utf8_percent_encode(&val, DEFAULT_ENCODE_SET).fmt(f)
    }
}

pub enum ProgressBarMode {
    Disabled,
    Request,
    Response,
    Both,
    Shared((Arc<ProgressBar>, u64, u64)),
}

impl ProgressBarMode {
    /// Returns if progress bars are generally enabled.
    pub fn active(&self) -> bool {
        match *self {
            ProgressBarMode::Disabled => false,
            _ => true,
        }
    }

    /// Returns whether a progress bar should be displayed for during upload.
    pub fn request(&self) -> bool {
        match *self {
            ProgressBarMode::Request | ProgressBarMode::Both => true,
            _ => false,
        }
    }

    /// Returns whether a progress bar should be displayed for during download.
    pub fn response(&self) -> bool {
        match *self {
            ProgressBarMode::Response | ProgressBarMode::Both => true,
            _ => false,
        }
    }
}

/// Helper for the API access.
pub struct Api {
    config: Arc<Config>,
    shared_handle: RefCell<curl::easy::Easy>,
}

/// Represents file contents temporarily
pub enum FileContents<'a> {
    FromPath(&'a Path),
    FromBytes(&'a [u8]),
}

/// Represents API errors.
#[derive(Debug)]
pub enum Error {
    Http(u32, String),
    Curl(curl::Error),
    Form(curl::FormError),
    Io(io::Error),
    Json(serde_json::Error),
    NotJson,
    ResourceNotFound(&'static str),
    BadApiUrl(String),
    NoDsn,
}

/// Shortcut alias for results of this module.
pub type ApiResult<T> = Result<T, Error>;

/// Represents an HTTP method that is used by the API.
#[derive(PartialEq, Debug)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
pub struct ApiRequest<'a> {
    handle: RefMut<'a, curl::easy::Easy>,
    headers: curl::easy::List,
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

impl Api {
    /// Creates a new API access helper.  For as long as it lives HTTP
    /// keepalive can be used.  When the object is recreated new
    /// connections will be established.
    pub fn new() -> Api {
        Api::with_config(Config::get_current())
    }

    /// Similar to `new` but uses a specific config.
    pub fn with_config(config: Arc<Config>) -> Api {
        Api {
            config: config,
            shared_handle: RefCell::new(curl::easy::Easy::new()),
        }
    }

    // Low Level Methods

    /// Create a new `ApiRequest` for the given HTTP method and URL.  If the
    /// URL is just a path then it's relative to the configured API host
    /// and authentication is automatically enabled.
    pub fn request<'a>(&'a self, method: Method, url: &str) -> ApiResult<ApiRequest<'a>> {
        let mut handle = self.shared_handle.borrow_mut();
        if !self.config.allow_keepalive() {
            handle.forbid_reuse(true).ok();
        }
        handle.reset();
        let mut ssl_opts = curl::easy::SslOpt::new();
        if self.config.disable_ssl_revocation_check() {
            ssl_opts.no_revoke(true);
        }
        handle.ssl_options(&ssl_opts)?;
        let (url, auth) = if url.starts_with("http://") || url.starts_with("https://") {
            (Cow::Borrowed(url), None)
        } else {
            (
                Cow::Owned(match self.config.get_api_endpoint(url) {
                    Ok(rv) => rv,
                    Err(err) => return Err(Error::BadApiUrl(err.to_string())),
                }),
                self.config.get_auth(),
            )
        };

        // the proxy url is discovered from the http_proxy envvar.
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

        ApiRequest::new(handle, method, &url, auth)
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
    pub fn download(&self, url: &str, dst: &mut fs::File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, &url)?
            .follow_location(true)?
            .send_into(dst)
    }

    /// Convenience method that downloads a file into the given file object
    /// and show a progress bar
    pub fn download_with_progress(&self, url: &str, dst: &mut fs::File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, &url)?
            .follow_location(true)?
            .progress_bar_mode(ProgressBarMode::Response)?
            .send_into(dst)
    }

    /// Convenience method that waits for a few seconds until a resource
    /// becomes available.
    pub fn wait_until_available(&self, url: &str, duration: Duration) -> ApiResult<bool> {
        let started = Utc::now();
        loop {
            match self.request(Method::Get, &url)?.send() {
                Ok(_) => return Ok(true),
                Err(err) => match err {
                    Error::Http(..) | Error::Curl(..) => {}
                    err => return Err(err),
                },
            }
            thread::sleep(Duration::milliseconds(500).to_std().unwrap());
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

    /// Lists all the release file for the given `release`.
    pub fn list_release_files(
        &self,
        org: &str,
        project: Option<&str>,
        release: &str,
    ) -> ApiResult<Vec<Artifact>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/files/",
                PathArg(org),
                PathArg(project),
                PathArg(release)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/",
                PathArg(org),
                PathArg(release)
            )
        };
        self.get(&path)?.convert_rnf("release")
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
            resp.to_result().map(|_| true)
        }
    }

    /// Uploads a new release file.  The file is loaded directly from the file
    /// system and uploaded as `name`.
    pub fn upload_release_file(
        &self,
        org: &str,
        project: Option<&str>,
        version: &str,
        contents: FileContents,
        name: &str,
        dist: Option<&str>,
        headers: Option<&[(String, String)]>,
    ) -> ApiResult<Option<Artifact>> {
        let path = if let Some(project) = project {
            format!(
                "/projects/{}/{}/releases/{}/files/",
                PathArg(org),
                PathArg(project),
                PathArg(version)
            )
        } else {
            format!(
                "/organizations/{}/releases/{}/files/",
                PathArg(org),
                PathArg(version)
            )
        };
        let mut form = curl::easy::Form::new();
        match contents {
            FileContents::FromPath(path) => {
                form.part("file").file(path).add()?;
            }
            FileContents::FromBytes(bytes) => {
                let filename = Path::new(name)
                    .file_name()
                    .and_then(|x| x.to_str())
                    .unwrap_or("unknown.bin");
                form.part("file").buffer(filename, bytes.to_vec()).add()?;
            }
        }
        form.part("name").contents(name.as_bytes()).add()?;
        if let Some(dist) = dist {
            form.part("dist").contents(dist.as_bytes()).add()?;
        }

        if let Some(headers) = headers {
            for &(ref key, ref value) in headers {
                form.part("header")
                    .contents(format!("{}:{}", key, value).as_bytes())
                    .add()?;
            }
        }

        let resp = self.request(Method::Post, &path)?
            .with_form_data(form)?
            .progress_bar_mode(ProgressBarMode::Request)?
            .send()?;
        if resp.status() == 409 {
            Ok(None)
        } else {
            resp.convert_rnf("release")
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
                .convert_rnf("organization or project")
        } else {
            let path = format!("/organizations/{}/releases/", PathArg(org));
            self.post(&path, release)?.convert_rnf("organization")
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
                self.put(&path, release)?.convert_rnf("release")
            } else {
                let path = format!("/organizations/{}/releases/{}/", PathArg(org), PathArg(version));
                self.put(&path, release)?.convert_rnf("release")
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
        self.put(&path, &update)?.convert_rnf("release")
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
            resp.to_result().map(|_| true)
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
            self.get(&path)?.convert_rnf("organization or project")
        } else {
            let path = format!("/organizations/{}/releases/", PathArg(org));
            self.get(&path)?.convert_rnf("organization")
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
            .convert_rnf("organization or release")
    }

    /// Lists all deploys for a release
    pub fn list_deploys(&self, org: &str, version: &str) -> ApiResult<Vec<Deploy>> {
        let path = format!(
            "/organizations/{}/releases/{}/deploys/",
            PathArg(org),
            PathArg(version)
        );
        self.get(&path)?.convert_rnf("organization or release")
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
            .to_result()
            .map(|_| true)
    }

    /// Finds the latest release for sentry-cli on GitHub.
    pub fn get_latest_sentrycli_release(&self) -> ApiResult<Option<SentryCliRelease>> {
        let resp = self.get("https://api.github.com/repos/getsentry/sentry-cli/releases/latest")?;
        let ref_name = format!(
            "sentry-cli-{}-{}{}",
            utils::capitalize_string(PLATFORM),
            ARCH,
            EXT
        );
        info!("Looking for file named: {}", ref_name);

        if resp.status() == 404 {
            Ok(None)
        } else {
            let info: GitHubRelease = resp.to_result()?.convert()?;
            for asset in info.assets {
                info!("Found asset {}", asset.name);
                if asset.name == ref_name {
                    return Ok(Some(SentryCliRelease {
                        version: info.tag_name,
                        download_url: asset.browser_download_url,
                    }));
                }
            }
            warn!("Unable to find release file");
            Ok(None)
        }
    }

    /// Given a list of checksums for Dsym files this returns a list of those
    /// that do not exist for the project yet.
    pub fn find_missing_dsym_checksums(
        &self,
        org: &str,
        project: &str,
        checksums: &Vec<&str>,
    ) -> ApiResult<HashSet<String>> {
        let mut url = format!(
            "/projects/{}/{}/files/dsyms/unknown/?",
            PathArg(org),
            PathArg(project)
        );
        for (idx, checksum) in checksums.iter().enumerate() {
            if idx > 0 {
                url.push('&');
            }
            url.push_str("checksums=");
            url.push_str(checksum);
        }

        let state: MissingChecksumsResponse = self.get(&url)?.convert()?;
        Ok(state.missing)
    }

    /// Uploads a dsym file from the given path.
    pub fn upload_dsyms(&self, org: &str, project: &str, file: &Path) -> ApiResult<Vec<DSymFile>> {
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
                checksums: checksums,
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
        manifest: &utils::AndroidManifest,
        checksums: Vec<String>,
    ) -> ApiResult<Option<AssociateDsymsResponse>> {
        self.associate_dsyms(
            org,
            project,
            &AssociateDsyms {
                platform: "android".to_string(),
                checksums: checksums,
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
        let resp = self.request(Method::Post, &path)?
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
        let resp = self.request(Method::Post, &path)?.send()?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.to_result().map(|_| true)
        }
    }

    /// List all projects associated with an organization
    pub fn list_organization_projects(&self, org: &str) -> ApiResult<Vec<Project>> {
        self.get(&format!("/organizations/{}/projects/", PathArg(org)))?
            .convert_rnf("organization")
    }

    /// List all repos associated with an organization
    pub fn list_organization_repos(&self, org: &str) -> ApiResult<Vec<Repo>> {
        let path = format!("/organizations/{}/repos/", PathArg(org));
        let resp = self.request(Method::Get, &path)?.send()?;
        if resp.status() == 404 {
            Ok(vec![])
        } else {
            Ok(resp.convert()?)
        }
    }

    /// Sends a single Sentry event.  The return value is the ID of the event
    /// that was sent.
    pub fn send_event(&self, dsn: &Dsn, event: &Event) -> ApiResult<String> {
        let event: EventInfo = self.request(Method::Post, &dsn.get_submit_url())?
            .with_header("X-Sentry-Auth", &dsn.get_auth_header(event.timestamp))?
            .with_json_body(&event)?
            .send()?
            .convert()?;
        Ok(event.id)
    }
}

fn send_req<W: Write>(
    handle: &mut curl::easy::Easy,
    out: &mut W,
    body: Option<Vec<u8>>,
    progress_bar_mode: ProgressBarMode,
) -> ApiResult<(u32, Vec<String>)> {
    match body {
        Some(body) => {
            let mut body = &body[..];
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
    read: &mut FnMut(&mut [u8]) -> usize,
) -> ApiResult<(u32, Vec<String>)> {
    if progress_bar_mode.active() {
        handle.progress(true)?;
    }

    // enable verbose mode
    handle.verbose(true)?;

    let mut headers = Vec::new();
    let pb: Rc<RefCell<Option<ProgressBar>>> = Rc::new(RefCell::new(None));
    {
        let mut headers = &mut headers;
        let mut handle = handle.transfer();

        if let ProgressBarMode::Shared((pb_progress, len, offset)) = progress_bar_mode {
            handle.progress_function(move |_, _, total, uploaded| {
                if uploaded > 0f64 && uploaded < total {
                    let position = offset + (uploaded / total * (len as f64)) as u64;
                    pb_progress.set_position(position);
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
                            *pb = Some(utils::make_byte_progress_bar(up_len));
                        }
                        pb.as_ref().unwrap().set_position(up_pos);
                    } else if pb.is_some() {
                        pb.take().unwrap().finish_and_clear();
                    }
                }
                if down_len > 0 && progress_bar_mode.response() {
                    if down_pos < down_len {
                        if pb.is_none() {
                            *pb = Some(utils::make_byte_progress_bar(down_len));
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

impl<'a> ApiRequest<'a> {
    fn new(
        mut handle: RefMut<'a, curl::easy::Easy>,
        method: Method,
        url: &str,
        auth: Option<&Auth>,
    ) -> ApiResult<ApiRequest<'a>> {
        info!("request {} {}", method, url);

        let mut headers = curl::easy::List::new();
        headers.append("Expect:").ok();
        headers
            .append(&format!("User-Agent: sentry-cli/{}", VERSION))
            .ok();

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

        handle.url(&url)?;

        let request = ApiRequest {
            handle: handle,
            headers: headers,
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
        match *auth {
            Auth::Key(ref key) => {
                self.handle.username(key)?;
                info!("using key based authentication");
            }
            Auth::Token(ref token) => {
                self.headers
                    .append(&format!("Authorization: Bearer {}", token))?;
                info!("using token authentication");
            }
        }

        Ok(self)
    }

    /// adds a specific header to the request
    pub fn with_header(mut self, key: &str, value: &str) -> ApiResult<ApiRequest<'a>> {
        self.headers.append(&format!("{}: {}", key, value))?;
        Ok(self)
    }

    /// sets the JSON request body for the request.
    pub fn with_json_body<S: Serialize>(mut self, body: &S) -> ApiResult<ApiRequest<'a>> {
        let mut body_bytes: Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &body)?;
        info!("sending JSON data ({} bytes)", body_bytes.len());
        self.body = Some(body_bytes);
        self.headers.append("Content-Type: application/json")?;
        Ok(self)
    }

    /// attaches some form data to the request.
    pub fn with_form_data(mut self, form: curl::easy::Form) -> ApiResult<ApiRequest<'a>> {
        info!("sending form data");
        self.handle.httppost(form)?;
        self.body = None;
        Ok(self)
    }

    /// enables or disables redirects.  The default is off.
    pub fn follow_location(mut self, val: bool) -> ApiResult<ApiRequest<'a>> {
        info!("follow redirects: {}", val);
        self.handle.follow_location(val)?;
        Ok(self)
    }

    /// enables a progress bar.
    pub fn progress_bar_mode(mut self, mode: ProgressBarMode) -> ApiResult<ApiRequest<'a>> {
        self.progress_bar_mode = mode;
        Ok(self)
    }

    /// Sends the request and writes response data into the given file
    /// instead of the response object's in memory buffer.
    pub fn send_into<W: Write>(mut self, out: &mut W) -> ApiResult<ApiResponse> {
        self.handle.http_headers(self.headers)?;
        let (status, headers) = send_req(&mut self.handle, out, self.body, self.progress_bar_mode)?;
        info!("response: {}", status);
        Ok(ApiResponse {
            status: status,
            headers: headers,
            body: None,
        })
    }

    /// Sends the request and reads the response body into the response object.
    pub fn send(self) -> ApiResult<ApiResponse> {
        let mut out = vec![];
        let mut rv = self.send_into(&mut out)?;
        rv.body = Some(out);
        Ok(rv)
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
    pub fn to_result(self) -> ApiResult<ApiResponse> {
        if let Some(ref body) = self.body {
            info!("body: {}", String::from_utf8_lossy(body));
        }
        if self.ok() {
            return Ok(self);
        }
        if let Ok(err) = self.deserialize::<ErrorInfo>() {
            if let Some(detail) = err.detail.or(err.error) {
                fail!(Error::Http(self.status(), detail));
            }
        }
        if let Ok(value) = self.deserialize::<serde_json::Value>() {
            fail!(Error::Http(
                self.status(),
                format!("protocol error:\n\n{:#}", value)
            ));
        } else {
            fail!(Error::Http(self.status(), "generic error".into()));
        }
    }

    /// Deserializes the response body into the given type
    pub fn deserialize<T: DeserializeOwned>(&self) -> ApiResult<T> {
        if !self.is_json() {
            fail!(Error::NotJson);
        }
        Ok(serde_json::from_reader(match self.body {
            Some(ref body) => body,
            None => &b""[..],
        })?)
    }

    /// Like `deserialize` but consumes the response and will convert
    /// failed requests into proper errors.
    pub fn convert<T: DeserializeOwned>(self) -> ApiResult<T> {
        self.to_result().and_then(|x| x.deserialize())
    }

    /// Like convert but produces resource not found errors.
    pub fn convert_rnf<T: DeserializeOwned>(self, resource: &'static str) -> ApiResult<T> {
        if self.status() == 404 {
            Err(Error::ResourceNotFound(resource))
        } else {
            self.to_result().and_then(|x| x.deserialize())
        }
    }

    /// Iterates over the headers.
    #[allow(dead_code)]
    pub fn headers(&self) -> Headers {
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

    /// Returns true if the response is JSON.
    pub fn is_json(&self) -> bool {
        self.get_header("content-type")
            .and_then(|x| x.split(';').next())
            .unwrap_or("") == "application/json"
    }
}

fn log_headers(is_response: bool, data: &[u8]) {
    lazy_static! {
        static ref AUTH_RE: Regex = Regex::new(
            r"(?i)(authorization):\s*([\w]+)\s+(.*)").unwrap();
    }
    if let Ok(header) = str::from_utf8(data) {
        for line in header.lines() {
            if line.is_empty() {
                continue;
            }

            let replaced = AUTH_RE.replace_all(line, |caps: &Captures| {
                let info = if &caps[1].to_lowercase() == "basic" {
                    caps[3].split(':').next().unwrap().to_string()
                } else {
                    format!("{}***", &caps[3][..8])
                };
                format!("{}: {} {}", &caps[1], &caps[2], info)
            });
            info!("{} {}", if is_response { ">" } else { "<" }, replaced);
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "api error"
    }
}

impl From<curl::FormError> for Error {
    fn from(err: curl::FormError) -> Error {
        Error::Form(err)
    }
}

impl From<curl::Error> for Error {
    fn from(err: curl::Error) -> Error {
        Error::Curl(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Http(status, ref msg) => write!(f, "http error: {} ({})", msg, status),
            Error::Curl(ref err) => write!(f, "http error: {}", err),
            Error::Form(ref err) => write!(f, "http form error: {}", err),
            Error::Io(ref err) => write!(f, "io error: {}", err),
            Error::Json(ref err) => write!(f, "bad json: {}", err),
            Error::NotJson => write!(f, "not a JSON response"),
            Error::ResourceNotFound(res) => write!(f, "{} not found", res),
            Error::NoDsn => write!(f, "no dsn provided"),
            Error::BadApiUrl(ref msg) => write!(f, "{}", msg),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    detail: Option<String>,
    error: Option<String>,
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
#[derive(Deserialize, Debug)]
pub struct Artifact {
    pub id: String,
    pub sha1: String,
    pub name: String,
    pub size: u64,
    pub dist: Option<String>,
    pub headers: HashMap<String, String>,
}

impl Artifact {
    pub fn get_header<'a, 'b>(&'a self, key: &'b str) -> Option<&'a str> {
        let ikey = key.to_lowercase();
        for (k, v) in self.headers.iter() {
            if k.to_lowercase() == ikey {
                return Some(v.as_str());
            }
        }
        None
    }

    pub fn get_sourcemap_reference(&self) -> Option<&str> {
        utils::get_sourcemap_reference_from_headers(self.headers.iter())
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

/// Changes to a release
#[derive(Debug, Serialize, Default)]
pub struct UpdatedRelease {
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
}

/// Provides all release information from already existing releases
#[derive(Debug, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub url: Option<String>,
    #[serde(rename = "dateCreated")]
    pub date_created: DateTime<Utc>,
    #[serde(rename = "dateReleased")]
    pub date_released: Option<DateTime<Utc>>,
    #[serde(rename = "lastEvent")]
    pub last_event: Option<DateTime<Utc>>,
    #[serde(rename = "newGroups")]
    pub new_groups: u64,
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

/// Information about sentry CLI releases
pub struct SentryCliRelease {
    pub version: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct EventInfo {
    id: String,
}

/// Structure of DSym files.
#[derive(Debug, Deserialize)]
pub struct DSymFile {
    pub uuid: String,
    #[serde(rename = "objectName")]
    pub object_name: String,
    #[serde(rename = "cpuName")]
    pub cpu_name: String,
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
    missing: HashSet<String>,
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
                    rv.push(format!("id={}", id));
                }
            }
            IssueFilter::Status(ref status) => {
                rv.push(format!("status={}", status));
            }
        }
        Some(rv.join("&"))
    }
}

#[derive(Deserialize)]
pub struct AssociateDsymsResponse {
    #[serde(rename = "associatedDsymFiles")]
    pub associated_dsyms: Vec<DSymFile>,
}

#[derive(Deserialize, Debug)]
pub struct Team {
    pub id: String,
    pub slug: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub team: Team,
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
