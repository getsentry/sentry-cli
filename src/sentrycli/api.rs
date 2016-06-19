use std::io;
use std::fs;
use std::io::{Read, Write};
use std::fmt;
use std::path::Path;
use std::ascii::AsciiExt;
use std::collections::HashSet;

use serde::{Serialize, Deserialize};
use serde_json;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use curl;

use utils;
use event::Event;
use config::{Config, Auth};
use constants::{PLATFORM, ARCH, EXT, VERSION};


struct UrlArg<A: fmt::Display>(A);

impl<A: fmt::Display> fmt::Display for UrlArg<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = format!("{}", self.0);
        utf8_percent_encode(&val, DEFAULT_ENCODE_SET).fmt(f)
    }
}

pub struct Api<'a> {
    config: &'a Config,
    handle: curl::easy::Easy,
}

enum Body {
    Empty,
    Json(Vec<u8>),
    Form(curl::easy::Form),
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

#[derive(Clone, Debug)]
pub struct ApiResponse {
    status: u32,
    headers: Vec<String>,
    body: Vec<u8>,
}

impl<'a> Api<'a> {
    pub fn new(config: &'a Config) -> Api<'a> {
        Api {
            config: config,
            handle: curl::easy::Easy::new(),
        }
    }

    pub fn get_auth_info(&mut self) -> ApiResult<AuthInfo> {
        Ok(self.get("/")?.convert()?)
    }

    pub fn list_release_files(&mut self, org: &str, project: &str,
                              release: &str) -> ApiResult<Vec<Artifact>> {
        Ok(self.get(&format!("/projects/{}/{}/releases/{}/files/",
                             UrlArg(org), UrlArg(project),
                             UrlArg(release)))?.convert()?)
    }

    pub fn delete_release_file(&mut self, org: &str, project: &str, version: &str,
                               file_id: &str)
        -> ApiResult<bool>
    {
        let resp = self.delete(&format!("/projects/{}/{}/releases/{}/files/{}/",
                                        UrlArg(org), UrlArg(project),
                                        UrlArg(version), UrlArg(file_id)))?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.to_result().map(|_| true)
        }
    }

    pub fn upload_release_file(&mut self, org: &str, project: &str,
                               version: &str, local_path: &Path, name: &str)
        -> ApiResult<Option<Artifact>>
    {
        self.handle.reset();
        let mut form = curl::easy::Form::new();
        form.part("file").file(local_path).add()?;
        // XXX: guess type here
        form.part("header")
            .contents(b"Content-Type:application/octet-stream").add()?;
        form.part("name").contents(name.as_bytes()).add()?;

        let headers = self.make_headers();
        let resp = self.req(&format!("/projects/{}/{}/releases/{}/files/",
                                     UrlArg(org), UrlArg(project),
                                     UrlArg(version)), Body::Form(form),
                                     headers)?;
        if resp.status() == 409 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    pub fn new_release(&mut self, org: &str, project: &str,
                       release: &NewRelease) -> ApiResult<ReleaseInfo> {
        Ok(self.post(&format!("/projects/{}/{}/releases/",
                              UrlArg(org), UrlArg(project)), release)?.convert()?)
    }

    pub fn delete_release(&mut self, org: &str, project: &str, version: &str)
        -> ApiResult<bool>
    {
        let resp = self.delete(&format!("/projects/{}/{}/releases/{}/",
                                        UrlArg(org), UrlArg(project),
                                        UrlArg(version)))?;
        if resp.status() == 404 {
            Ok(false)
        } else {
            resp.to_result().map(|_| true)
        }
    }

    pub fn get_release(&mut self, org: &str, project: &str, version: &str)
        -> ApiResult<Option<ReleaseInfo>> {
        let resp = self.get(&format!("/projects/{}/{}/releases/{}/",
                                     UrlArg(org), UrlArg(project), UrlArg(version)))?;
        if resp.status() == 404 {
            Ok(None)
        } else {
            resp.convert()
        }
    }

    pub fn list_releases(&mut self, org: &str, project: &str)
        -> ApiResult<Vec<ReleaseInfo>> {
        Ok(self.get(&format!("/projects/{}/{}/releases/",
                             UrlArg(org), UrlArg(project)))?.convert()?)
    }

    pub fn get_latest_sentrycli_release(&mut self)
        -> ApiResult<Option<SentryCliRelease>>
    {
        let resp = self.get("https://api.github.com/repos/getsentry/sentry-cli/releases/latest")?;
        let ref_name = format!("sentry-cli-{}-{}{}",
           utils::capitalize_string(PLATFORM), ARCH, EXT);

        if resp.status() == 404 {
            Ok(None)
        } else {
            let resp = resp.to_result()?;
            let info : GitHubRelease = resp.convert()?;
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

    pub fn find_missing_dsym_checksums(&mut self, org: &str, project: &str,
                                       checksums: &Vec<&str>)
        -> ApiResult<HashSet<String>>
    {
        let mut url = format!("/projects/{}/{}/files/dsyms/unknown/?",
                              UrlArg(org), UrlArg(project));
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

    pub fn upload_dsyms(&mut self, org: &str, project: &str, file: &Path)
        -> ApiResult<Vec<DSymFile>>
    {
        self.handle.reset();
        let mut form = curl::easy::Form::new();
        form.part("file").file(file).add()?;
        let headers = self.make_headers();
        Ok(self.req(&format!("/projects/{}/{}/files/dsyms/",
                             UrlArg(org), UrlArg(project)), Body::Form(form),
                             headers)?.convert()?)
    }

    pub fn send_event(&mut self, event: &Event) -> ApiResult<String> {
        self.handle.reset();
        let dsn = self.config.dsn.as_ref().ok_or(Error::NoDsn)?;
        let mut headers = self.make_headers();
        headers.append(&format!("X-Sentry-Auth: {}",
                                dsn.get_auth_header(event.timestamp)))?;

        self.handle.custom_request("POST")?;
        let mut body_bytes : Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &event)?;
        let event : EventInfo = self.req(&dsn.get_submit_url(),
            Body::Json(body_bytes), headers)?.convert()?;
        Ok(event.id)
    }

    pub fn get(&mut self, path: &str) -> ApiResult<ApiResponse> {
        self.handle.reset();
        self.handle.get(true)?;
        let headers = self.make_headers();
        self.req(path, Body::Empty, headers)
    }

    pub fn delete(&mut self, path: &str) -> ApiResult<ApiResponse> {
        self.handle.reset();
        self.handle.custom_request("DELETE")?;
        let headers = self.make_headers();
        self.req(path, Body::Empty, headers)
    }

    pub fn post<S: Serialize>(&mut self, path: &str, body: &S) -> ApiResult<ApiResponse> {
        self.handle.reset();
        self.handle.custom_request("POST")?;
        let mut body_bytes : Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &body)?;
        let headers = self.make_headers();
        self.req(path, Body::Json(body_bytes), headers)
    }

    pub fn download(&mut self, url: &str, dst: &mut fs::File) -> ApiResult<()> {
        self.handle.reset();
        self.handle.url(&url)?;
        let headers = self.make_headers();
        self.handle.http_headers(headers)?;
        self.handle.follow_location(true)?;
        self.handle.progress(true)?;
        let (_, _) = send_req(&mut self.handle, dst, None)?;
        Ok(())
    }

    fn req(&mut self, path: &str, body: Body, mut headers: curl::easy::List)
        -> ApiResult<ApiResponse>
    {
        let (url, want_auth) = if path.starts_with("http://") ||
                                  path.starts_with("https://") {
            (path.into(), false)
        } else {
            (format!("{}/api/0/{}",
                     self.config.url.trim_right_matches('/'),
                     path.trim_left_matches('/')), true)
        };
        self.handle.url(&url)?;

        if want_auth {
            match self.config.auth {
                Auth::Key(ref key) => {
                    self.handle.username(key)?;
                }
                Auth::Token(ref token) => {
                    headers.append(&format!("Authorization: Bearer {}", token))?;
                }
                Auth::Unauthorized => {}
            }
        }

        let body_bytes = match body {
            Body::Empty => None,
            Body::Json(bytes) => {
                headers.append("Content-Type: application/json")?;
                Some(bytes)
            },
            Body::Form(form) => {
                self.handle.httppost(form)?;
                None
            }
        };

        self.handle.http_headers(headers)?;
        let mut out : Vec<u8> = vec![];
        let (status, headers) = send_req(&mut self.handle, &mut out, body_bytes)?;
        Ok(ApiResponse {
            status: status,
            headers: headers,
            body: out,
        })
    }

    fn make_headers(&self) -> curl::easy::List {
        let mut headers = curl::easy::List::new();
        headers.append("Expect:").ok();
        headers.append(&format!("User-Agent: sentry-cli/{}", VERSION)).ok();
        headers
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
        Ok(serde_json::from_reader(&self.body[..])?)
    }

    /// Like `deserialize` but consumes the response and will convert
    /// failed requests into proper errors.
    pub fn convert<T: Deserialize>(self) -> ApiResult<T> {
        self.to_result().and_then(|x| {
            x.deserialize()
        })
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
