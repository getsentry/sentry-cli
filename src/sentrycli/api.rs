use std::io;
use std::fs;
use std::io::{Read, Write};
use std::fmt;
use std::cell::{RefMut, RefCell};
use std::path::Path;
use std::ascii::AsciiExt;
use std::collections::HashSet;
use std::borrow::Cow;

use serde::{Serialize, Deserialize};
use serde_json;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use curl;

use utils;
use event::Event;
use config::{Config, Auth};
use constants::{PLATFORM, ARCH, EXT, VERSION};


struct PathArg<A: fmt::Display>(A);

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

pub struct Api<'a> {
    config: &'a Config,
    shared_handle: RefCell<curl::easy::Easy>,
}

#[derive(Debug)]
pub enum Error {
    Http(u32, String),
    Curl(curl::Error),
    Form(curl::FormError),
    Io(io::Error),
    Json(serde_json::Error),
    NoDsn,
}

pub type ApiResult<T> = Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Method {
    Get,
    Post,
    Delete,
}

pub struct ApiRequest<'a> {
    handle: RefMut<'a, curl::easy::Easy>,
    headers: curl::easy::List,
    body: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct ApiResponse {
    status: u32,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl<'a> Api<'a> {
    pub fn new(config: &'a Config) -> Api<'a> {
        Api {
            config: config,
            shared_handle: RefCell::new(curl::easy::Easy::new()),
        }
    }

    // Low Level Methods

    /// Create a new `ApiRequest` for the given HTTP method and URL.  If the
    /// URL is just a path then it's relative to the configured API host
    /// and authentication is automatically enabled.
    pub fn request(&'a self, method: Method, url: &str) -> ApiResult<ApiRequest<'a>> {
        let mut handle = self.shared_handle.borrow_mut();
        if !self.config.allow_keepalive() {
            handle.forbid_reuse(true).ok();
        }
        let (url, auth) = if url.starts_with("http://") || url.starts_with("https://") {
            (Cow::Borrowed(url), None)
        } else {
            (
                Cow::Owned(format!("{}/api/0/{}", self.config.url.trim_right_matches('/'),
                                   url.trim_left_matches('/'))),
                self.config.auth.as_ref()
            )
        };

        if let Some(proxy_url) = self.config.get_proxy_url() {
            handle.proxy(proxy_url)?;
        }
        if let Some(proxy_username) = self.config.get_proxy_username() {
            handle.proxy_username(proxy_username)?;
        }
        if let Some(proxy_password) = self.config.get_proxy_password() {
            handle.proxy_password(proxy_password)?;
        }
        handle.ssl_verify_host(self.config.should_verify_ssl())?;

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
        self.request(Method::Post, path)?.with_json_body(body)?.send()
    }

    /// Convenience method that downloads a file into the given file object.
    pub fn download(&self, url: &str, dst: &mut fs::File) -> ApiResult<ApiResponse> {
        self.request(Method::Get, &url)?.follow_location(true)?.send_into(dst)
    }

    // High Level Methods

    pub fn get_auth_info(&self) -> ApiResult<AuthInfo> {
        self.get("/")?.convert()
    }

    pub fn list_release_files(&self, org: &str, project: &str,
                              release: &str) -> ApiResult<Vec<Artifact>> {
        self.get(&format!("/projects/{}/{}/releases/{}/files/",
                          PathArg(org), PathArg(project),
                          PathArg(release)))?.convert()
    }

    pub fn delete_release_file(&self, org: &str, project: &str, version: &str,
                               file_id: &str)
        -> ApiResult<bool>
    {
        let resp = self.delete(&format!("/projects/{}/{}/releases/{}/files/{}/",
                                        PathArg(org), PathArg(project),
                                        PathArg(version), PathArg(file_id)))?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.to_result().map(|_| true)
        }
    }

    pub fn upload_release_file(&self, org: &str, project: &str,
                               version: &str, local_path: &Path, name: &str)
        -> ApiResult<Option<Artifact>>
    {
        let path = format!("/projects/{}/{}/releases/{}/files/",
                           PathArg(org), PathArg(project),
                           PathArg(version));
        let mut form = curl::easy::Form::new();
        form.part("file").file(local_path).add()?;
        form.part("name").contents(name.as_bytes()).add()?;

        let resp = self.request(Method::Post, &path)?.with_form_data(form)?.send()?;
        if resp.status() == 409 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    pub fn new_release(&self, org: &str, project: &str,
                       release: &NewRelease) -> ApiResult<ReleaseInfo> {
        Ok(self.post(&format!("/projects/{}/{}/releases/",
                              PathArg(org), PathArg(project)), release)?.convert()?)
    }

    pub fn delete_release(&self, org: &str, project: &str, version: &str)
        -> ApiResult<bool>
    {
        let resp = self.delete(&format!("/projects/{}/{}/releases/{}/",
                                        PathArg(org), PathArg(project),
                                        PathArg(version)))?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.to_result().map(|_| true)
        }
    }

    pub fn get_release(&self, org: &str, project: &str, version: &str)
        -> ApiResult<Option<ReleaseInfo>> {
        let resp = self.get(&format!("/projects/{}/{}/releases/{}/",
                                     PathArg(org), PathArg(project), PathArg(version)))?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    pub fn list_releases(&self, org: &str, project: &str)
        -> ApiResult<Vec<ReleaseInfo>> {
        Ok(self.get(&format!("/projects/{}/{}/releases/",
                             PathArg(org), PathArg(project)))?.convert()?)
    }

    pub fn get_latest_sentrycli_release(&self)
        -> ApiResult<Option<SentryCliRelease>>
    {
        let resp = self.get("https://api.github.com/repos/getsentry/sentry-cli/releases/latest")?;
        let ref_name = format!("sentry-cli-{}-{}{}",
           utils::capitalize_string(PLATFORM), ARCH, EXT);

        if resp.status() == 404 {
            Ok(None)
        } else {
            let info : GitHubRelease = resp.to_result()?.convert()?;
            for asset in info.assets {
                if asset.name == ref_name {
                    return Ok(Some(SentryCliRelease {
                        version: info.tag_name,
                        download_url: asset.browser_download_url,
                    }));
                }
            }
            Ok(None)
        }
    }

    pub fn find_missing_dsym_checksums(&self, org: &str, project: &str,
                                       checksums: &Vec<&str>)
        -> ApiResult<HashSet<String>>
    {
        let mut url = format!("/projects/{}/{}/files/dsyms/unknown/?",
                              PathArg(org), PathArg(project));
        for (idx, checksum) in checksums.iter().enumerate() {
            if idx > 0 {
                url.push('&');
            }
            url.push_str("checksums=");
            url.push_str(checksum);
        }

        let state : MissingChecksumsResponse = self.get(&url)?.convert()?;
        Ok(state.missing)
    }

    pub fn upload_dsyms(&self, org: &str, project: &str, file: &Path)
        -> ApiResult<Vec<DSymFile>>
    {
        let path = format!("/projects/{}/{}/files/dsyms/", PathArg(org), PathArg(project));
        let mut form = curl::easy::Form::new();
        form.part("file").file(file).add()?;
        self.request(Method::Post, &path)?.with_form_data(form)?.send()?.convert()
    }

    pub fn send_event(&self, event: &Event) -> ApiResult<String> {
        let dsn = self.config.dsn.as_ref().ok_or(Error::NoDsn)?;
        let event : EventInfo = self.request(Method::Post, &dsn.get_submit_url())?
            .with_header("X-Sentry-Auth", &dsn.get_auth_header(event.timestamp))?
            .with_json_body(&event)?
            .send()?.convert()?;
        Ok(event.id)
    }
}

fn send_req<W: Write>(handle: &mut curl::easy::Easy,
                      out: &mut W, body: Option<Vec<u8>>)
    -> ApiResult<(u32, Vec<String>)>
{
    match body {
        Some(body) => {
            let mut body = &body[..];
            handle.upload(true)?;
            handle.in_filesize(body.len() as u64)?;
            handle_req(handle, out,
                       &mut |buf| body.read(buf).unwrap_or(0))
        },
        None => {
            handle_req(handle, out, &mut |_| 0)
        }
    }
}

fn handle_req<W: Write>(handle: &mut curl::easy::Easy,
                        out: &mut W,
                        read: &mut FnMut(&mut [u8]) -> usize)
    -> ApiResult<(u32, Vec<String>)>
{
    let mut headers = Vec::new();
    {
        let mut handle = handle.transfer();
        handle.read_function(|buf| Ok(read(buf)))?;
        handle.write_function(|data| {
            Ok(match out.write_all(data) {
                Ok(_) => data.len(),
                Err(_) => 0,
            })
        })?;
        handle.header_function(|data| {
            headers.push(String::from_utf8_lossy(data).into_owned());
            true
        })?;
        handle.perform()?;
    }

    Ok((handle.response_code()?, headers))
}

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
                Some(i) => (&line[..i], line[i..].trim_left_matches(' ')),
                None => (&line[..], "")
            }
        })
    }
}

impl<'a> ApiRequest<'a> {
    fn new(mut handle: RefMut<'a, curl::easy::Easy>,
           method: Method, url: &str, auth: Option<&Auth>)
        -> ApiResult<ApiRequest<'a>>
    {
        let mut headers = curl::easy::List::new();
        headers.append("Expect:").ok();
        headers.append(&format!("User-Agent: sentry-cli/{}", VERSION)).ok();

        match method {
            Method::Get => handle.get(true)?,
            Method::Post => handle.custom_request("POST")?,
            Method::Delete => handle.custom_request("DELETE")?,
        }

        handle.url(&url)?;
        match auth {
            None => {},
            Some(&Auth::Key(ref key)) => {
                handle.username(key)?;
            }
            Some(&Auth::Token(ref token)) => {
                headers.append(&format!("Authorization: Bearer {}", token))?;
            }
        }

        Ok(ApiRequest {
            handle: handle,
            headers: headers,
            body: None,
        })
    }

    pub fn with_header(mut self, key: &str, value: &str) -> ApiResult<ApiRequest<'a>> {
        self.headers.append(&format!("{}: {}", key, value))?;
        Ok(self)
    }

    pub fn with_json_body<S: Serialize>(mut self, body: &S) -> ApiResult<ApiRequest<'a>> {
        let mut body_bytes : Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &body)?;
        self.body = Some(body_bytes);
        self.headers.append("Content-Type: application/json")?;
        Ok(self)
    }

    pub fn with_form_data(mut self, form: curl::easy::Form) -> ApiResult<ApiRequest<'a>> {
        self.handle.httppost(form)?;
        self.body = None;
        Ok(self)
    }

    pub fn follow_location(mut self, val: bool) -> ApiResult<ApiRequest<'a>> {
        self.handle.follow_location(val)?;
        Ok(self)
    }

    pub fn send_into<W: Write>(mut self, out: &mut W) -> ApiResult<ApiResponse> {
        self.handle.http_headers(self.headers)?;
        let (status, headers) = send_req(&mut self.handle, out, self.body)?;
        Ok(ApiResponse {
            status: status,
            headers: headers,
            body: None,
        })
    }

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
        if self.ok() {
            return Ok(self);
        }
        if let Ok(err) = self.deserialize::<ErrorInfo>() {
            if let Some(detail) = err.detail.or(err.error) {
                fail!(Error::Http(self.status(), detail));
            }
        }
        fail!(Error::Http(self.status(), "generic error".into()));
    }

    /// Deserializes the response body into the given type
    pub fn deserialize<T: Deserialize>(&self) -> ApiResult<T> {
        Ok(serde_json::from_reader(match self.body {
            Some(ref body) => body,
            None => &b""[..],
        })?)
    }

    /// Like `deserialize` but consumes the response and will convert
    /// failed requests into proper errors.
    pub fn convert<T: Deserialize>(self) -> ApiResult<T> {
        self.to_result().and_then(|x| x.deserialize())
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
            Error::Http(status, ref msg) => write!(f, "http error: {} ({})",
                                                   msg, status),
            Error::Curl(ref err) => write!(f, "http error: {}", err),
            Error::Form(ref err) => write!(f, "http form error: {}", err),
            Error::Io(ref err) => write!(f, "io error: {}", err),
            Error::Json(ref err) => write!(f, "bad json: {}", err),
            Error::NoDsn => write!(f, "no dsn provided"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    detail: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AuthDetails {
    pub scopes: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthInfo {
    pub auth: AuthDetails,
    pub user: Option<User>,
}

#[derive(Deserialize, Debug)]
pub struct Artifact {
    pub id: String,
    pub sha1: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct NewRelease {
    pub version: String,
    #[serde(rename="ref", skip_serializing_if="Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub url: Option<String>
}

#[derive(Debug, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    #[serde(rename="ref")]
    pub reference: Option<String>,
    pub url: Option<String>,
    #[serde(rename="dateCreated")]
    pub date_created: String,
    #[serde(rename="dateReleased")]
    pub date_released: Option<String>,
    #[serde(rename="newGroups")]
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

pub struct SentryCliRelease {
    pub version: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct EventInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct DSymFile {
    pub uuid: String,
    #[serde(rename="objectName")]
    pub object_name: String,
    #[serde(rename="cpuName")]
    pub cpu_name: String,
}

#[derive(Deserialize)]
struct MissingChecksumsResponse {
    missing: HashSet<String>,
}
