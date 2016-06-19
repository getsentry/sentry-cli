use std::io;
use std::fs;
use std::env;
use std::path::PathBuf;

use serde_json;
use clap::ArgMatches;
use url::Url;
use ini::Ini;
use hyper::method::Method;
use hyper::client::request::Request;
use hyper::client::response::Response;
use hyper::header::{Authorization, Basic, Bearer, ContentType, ContentLength, UserAgent};
use hyper::net::Fresh;

use CliResult;
use constants::{DEFAULT_URL, VERSION, PROTOCOL_VERSION};
use event::Event;

#[derive(Deserialize)]
pub struct EventInfo {
    id: String,
}

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

#[derive(Debug, Clone)]
pub struct Dsn {
    host: String,
    protocol: String,
    port: u16,
    client_id: String,
    secret: String,
    project_id: u64,
}

impl Dsn {

    fn from_str(dsn: &str) -> CliResult<Dsn> {
        let url = Url::parse(dsn)?;
        let project_id = if let Some(component_iter) = url.path_segments() {
            let components : Vec<_> = component_iter.collect();
            if components.len() != 1 {
                fail!("invalid dsn: invalid project ID");
            }
            components[0].parse().or(Err("invalid dsn: invalid project id"))?
        } else {
            fail!("invalid dsn: missing project ID");
        };
        if !(url.scheme() == "http" || url.scheme() == "https") {
            fail!(format!("invalid dsn: unknown protocol '{}'", url.scheme()));
        }

        if url.username() == "" {
            fail!("invalid dsn: missing client id");
        }

        Ok(Dsn {
            protocol: url.scheme().into(),
            host: url.host_str().ok_or("invalid dsn: missing host")?.into(),
            port: url.port_or_known_default().unwrap(),
            client_id: url.username().into(),
            secret: url.password().ok_or("invalid dsn: missing secret")?.into(),
            project_id: project_id,
        })
    }

    pub fn get_submit_url(&self) -> Url {
        Url::parse(&format!("{}://{}:{}/api/{}/store/",
                            self.protocol,
                            self.host,
                            self.port,
                            self.project_id)).unwrap()
    }

    pub fn get_auth_header(&self, ts: f64) -> String {
        format!("Sentry \
            sentry_timestamp={}, \
            sentry_client=sentry-cli/{}, \
            sentry_version={}, \
            sentry_key={}, \
            sentry_secret={}",
            ts,
            VERSION,
            PROTOCOL_VERSION,
            self.client_id,
            self.secret)
    }
}

#[derive(Clone)]
pub struct Config {
    pub filename: PathBuf,
    pub auth: Auth,
    pub url: String,
    pub dsn: Option<Dsn>,
    pub ini: Ini,
}

impl Config {

    pub fn from_cli_config() -> CliResult<Config> {
        let (filename, ini) = load_cli_config()?;
        Ok(Config {
            filename: filename,
            auth: get_default_auth(&ini),
            url: get_default_url(&ini),
            dsn: get_default_dsn(&ini)?,
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

    pub fn send_event(&self, event: &Event) -> CliResult<String> {
        let dsn = self.dsn.as_ref().ok_or("no dsn provided")?;
        let mut req = Request::new(Method::Post, dsn.get_submit_url())?;
        let mut body_bytes : Vec<u8> = vec![];
        serde_json::to_writer(&mut body_bytes, &event)?;
        {
            let mut headers = req.headers_mut();
            headers.set(UserAgent(format!("sentry-cli/{}", VERSION)));
            headers.set(ContentType(mime!(Application/Json)));
            headers.set(ContentLength(body_bytes.len() as u64));
            headers.set_raw("X-Sentry-Auth", vec![
                dsn.get_auth_header(event.timestamp).as_bytes().into()]);
        }
        let mut req = req.start()?;
        io::copy(&mut &body_bytes[..], &mut req)?;
        let mut resp = req.send()?;
        if !resp.status.is_success() {
            fail!(resp);
        } else {
            let event : EventInfo = serde_json::from_reader(&mut resp)?;
            Ok(event.id)
        }
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

fn get_default_dsn(ini: &Ini) -> CliResult<Option<Dsn>> {
    if let Some(ref val) = env::var("SENTRY_DSN").ok() {
        Ok(Some(Dsn::from_str(val)?))
    } else if let Some(val) = ini.get_from(Some("auth"), "dsn") {
        Ok(Some(Dsn::from_str(val)?))
    } else {
        Ok(None)
    }
}
