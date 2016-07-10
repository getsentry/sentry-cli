use std::io;
use std::fs;
use std::env;
use std::path::PathBuf;

use clap::ArgMatches;
use url::Url;
use ini::Ini;

use CliResult;
use constants::{DEFAULT_URL, VERSION, PROTOCOL_VERSION};

#[derive(Debug, Clone)]
pub enum Auth {
    Key(String),
    Token(String),
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

    pub fn get_submit_url(&self) -> String {
        format!("{}://{}:{}/api/{}/store/",
                self.protocol,
                self.host,
                self.port,
                self.project_id)
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
    pub auth: Option<Auth>,
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

    pub fn allow_keepalive(&self) -> bool {
        let val = self.ini.get_from(Some("http"), "keepalive");
        match val {
            // keepalive is broken on our dev server.  Since this makes local development
            // quite frustrating we disable keepalive (handle reuse) when we connect to
            // unprotected servers where it does not matter that much.
            None => !self.has_insecure_server(),
            Some(val) => val == "true",
        }
    }

    pub fn get_proxy_url(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_url")
    }

    pub fn get_proxy_username(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_username")
    }

    pub fn get_proxy_password(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_password")
    }

    pub fn has_insecure_server(&self) -> bool {
        self.url.starts_with("http://")
    }

    pub fn should_verify_ssl(&self) -> bool {
        let val = self.ini.get_from(Some("http"), "verify_ssl");
        match val {
            None => true,
            Some(val) => val == "true",
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

fn get_default_auth(ini: &Ini) -> Option<Auth> {
    if let Some(ref val) = env::var("SENTRY_AUTH_TOKEN").ok() {
        Some(Auth::Token(val.to_owned()))
    } else if let Some(ref val) = env::var("SENTRY_API_KEY").ok() {
        Some(Auth::Key(val.to_owned()))
    } else if let Some(val) = ini.get_from(Some("auth"), "token") {
        Some(Auth::Token(val.to_owned()))
    } else if let Some(val) = ini.get_from(Some("auth"), "api_key") {
        Some(Auth::Key(val.to_owned()))
    } else {
        None
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
