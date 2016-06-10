use std::io;
use std::fs;
use std::env;
use std::path::PathBuf;

use serde::Serialize;
use serde_json;
use clap::ArgMatches;
use url::Url;
use ini::Ini;
use hyper::method::Method;
use hyper::client::request::Request;
use hyper::client::response::Response;
use hyper::header::{Authorization, Basic, Bearer, ContentType, ContentLength};
use hyper::net::Fresh;

use CliResult;
use constants::DEFAULT_URL;

#[derive(Debug, Clone)]
pub enum Auth {
    Key(String),
    Token(String),
    Unauthorized
}

impl Auth {
    pub fn describe(&self) -> &str {
        match *self {
            Auth::Key(_) => "API Key",
            Auth::Token(_) => "Auth Token",
            Auth::Unauthorized => "Unauthorized",
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub filename: PathBuf,
    pub auth: Auth,
    pub url: String,
    pub ini: Ini,
}

impl Config {

    pub fn from_cli_config() -> CliResult<Config> {
        let (filename, ini) = load_cli_config()?;
        Ok(Config {
            filename: filename,
            auth: get_default_auth(&ini),
            url: get_default_url(&ini),
            ini: ini,
        })
    }

    pub fn prepare_api_request(&self, method: Method, path: &str)
        -> CliResult<Request<Fresh>>
    {
        let url = Url::parse(&format!(
            "{}/api/0{}", self.url.trim_right_matches("/"), path))?;
        let mut req = Request::new(method, url)?;
        {
            match self.auth {
                Auth::Key(ref api_key) => {
                    req.headers_mut().set(Authorization(Basic {
                        username: api_key.clone(),
                        password: None
                    }));
                },
                Auth::Token(ref token) => {
                    req.headers_mut().set(Authorization(Bearer {
                        token: token.clone()
                    }));
                },
                Auth::Unauthorized => {},
            }
        }
        Ok(req)
    }

    pub fn api_request(&self, method: Method, path: &str)
        -> CliResult<Response>
    {
        let req = self.prepare_api_request(method, path)?;
        Ok(req.start()?.send()?)
    }

    pub fn json_api_request<T: Serialize>(&self, method: Method, path: &str, body: &T)
        -> CliResult<Response>
    {
        let mut req = self.prepare_api_request(method, path)?;
        let mut body_bytes : Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &body)?;

        {
            let mut headers = req.headers_mut();
            headers.set(ContentType(mime!(Application/Json)));
            headers.set(ContentLength(body_bytes.len() as u64));
        }

        let mut req = req.start()?;
        io::copy(&mut &body_bytes[..], &mut req)?;
        Ok(req.send()?)
    }

    pub fn get_org_and_project(&self, matches: &ArgMatches) -> CliResult<(String, String)> {
        Ok((
            matches
                .value_of("org").map(|x| x.to_owned())
                .or_else(|| env::var("SENTRY_ORG").ok())
                .or_else(|| self.ini.get_from(Some("defaults"), "org").map(|x| x.to_owned()))
                .ok_or("An organization slug is required (provide with --org)")?,
            matches
                .value_of("project").map(|x| x.to_owned())
                .or_else(|| env::var("SENTRY_PROJECT").ok())
                .or_else(|| self.ini.get_from(Some("defaults"), "project").map(|x| x.to_owned()))
                .ok_or("A project slug is required (provide with --project)")?
        ))
    }

    pub fn get_org_and_project_defaults(&self) -> (Option<String>, Option<String>) {
        (
            env::var("SENTRY_ORG").ok()
                .or_else(|| self.ini.get_from(Some("defaults"), "org").map(|x| x.to_owned())),
            env::var("SENTRY_PROJECT").ok()
                .or_else(|| self.ini.get_from(Some("defaults"), "project").map(|x| x.to_owned()))
        )
    }
}
fn find_project_config_file() -> Option<PathBuf> {
    env::current_dir().ok().and_then(|mut path| {
        loop {
            path.push(".sentryclirc");
            if path.exists() {
                return Some(path);
            }
            path.pop();
            if !path.pop() {
                return None;
            }
        }
    })
}

fn load_cli_config() -> CliResult<(PathBuf, Ini)> {
    let mut home_fn = env::home_dir().ok_or("Could not find home dir")?;
    home_fn.push(".sentryclirc");

    let mut rv = match fs::File::open(&home_fn) {
        Ok(mut file) => Ini::read_from(&mut file)?,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ini::new()
            } else {
                fail!(err);
            }
        }
    };

    if let Some(project_config_path) = find_project_config_file() {
        let ini = Ini::read_from(&mut fs::File::open(&project_config_path)?)?;
        for (section, props) in ini.iter() {
            for (key, value) in props {
                rv.set_to(section.clone(), key.clone(), value.to_owned());
            }
        }
        Ok((project_config_path, rv))
    } else {
        Ok((home_fn, rv))
    }
}

fn get_default_auth(ini: &Ini) -> Auth {
    if let Some(ref val) = env::var("SENTRY_AUTH_TOKEN").ok() {
        Auth::Token(val.to_owned())
    } else if let Some(ref val) = env::var("SENTRY_API_KEY").ok() {
        Auth::Key(val.to_owned())
    } else if let Some(val) = ini.get_from(Some("auth"), "token") {
        Auth::Token(val.to_owned())
    } else if let Some(val) = ini.get_from(Some("auth"), "api_key") {
        Auth::Key(val.to_owned())
    } else {
        Auth::Unauthorized
    }
}

fn get_default_url(ini: &Ini) -> String {
    if let Some(ref val) = env::var("SENTRY_URL").ok() {
        val.to_owned()
    } else if let Some(val) = ini.get_from(Some("defaults"), "url") {
        val.to_owned()
    } else {
        DEFAULT_URL.to_owned()
    }
}
