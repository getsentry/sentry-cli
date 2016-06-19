use std::io;
use std::io::Read;
use std::fmt;
use std::ascii::AsciiExt;

use serde::Deserialize;
use serde_json;

use curl;

use config::{Config, Auth};


pub struct Api<'a> {
    config: &'a Config,
    handle: curl::easy::Easy,
}

#[derive(Debug)]
pub enum Error {
    Http(u32, String),
    Curl(curl::Error),
    Io(io::Error),
    Json(serde_json::Error),
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

    pub fn get(&mut self, path: &str) -> ApiResult<ApiResponse> {
        self.handle.get(true)?;
        self.request(path, None)
    }

    fn request(&mut self, path: &str, body: Option<&[u8]>) -> ApiResult<ApiResponse> {
        self.handle.url(&format!("{}/api/0/{}",
            self.config.url.trim_right_matches('/'),
            path.trim_left_matches('/')))?;
        let mut headers = curl::easy::List::new();
        match self.config.auth {
            Auth::Key(ref key) => {
                self.handle.username(key)?;
            }
            Auth::Token(ref token) => {
                headers.append(&format!("Authorization: Bearer {}", token))?;
            }
            Auth::Unauthorized => {}
        }
        self.handle.http_headers(headers)?;
        match body {
            Some(mut body) => {
                self.handle.upload(true)?;
                self.handle.in_filesize(body.len() as u64)?;
                handle_req(&mut self.handle, &mut |buf| body.read(buf).unwrap_or(0))
            },
            None => {
                handle_req(&mut self.handle, &mut |_| 0)
            }
        }
    }
}

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
    pub fn headers(&self) -> Headers {
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
}

fn handle_req(handle: &mut curl::easy::Easy,
              read: &mut FnMut(&mut [u8]) -> usize) -> ApiResult<ApiResponse> {
    let mut headers = Vec::new();
    let mut body = Vec::new();
    {
        let mut handle = handle.transfer();
        handle.read_function(|buf| Ok(read(buf)))?;
        handle.write_function(|data| {
            body.extend_from_slice(data);
            Ok(data.len())
        })?;
        handle.header_function(|data| {
            headers.push(String::from_utf8_lossy(data).into_owned());
            true
        })?;
        handle.perform()?;
    }

    Ok(ApiResponse {
        status: handle.response_code()?,
        headers: headers,
        body: body,
    })
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
            Error::Io(ref err) => write!(f, "io error: {}", err),
            Error::Json(ref err) => write!(f, "bad json: {}", err),
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
