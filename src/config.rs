//! This module implements config access.
use std::io;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;

use dotenv;
use log;
use java_properties;
use clap::ArgMatches;
use url::Url;
use ini::Ini;

use prelude::*;
use constants::{DEFAULT_URL, VERSION, PROTOCOL_VERSION};
use utils::Logger;

/// Represents the auth information
#[derive(Debug, Clone)]
pub enum Auth {
    Key(String),
    Token(String),
}

/// Represents a DSN
#[derive(Debug, Clone)]
pub struct Dsn {
    pub host: String,
    pub protocol: String,
    pub port: u16,
    pub client_id: String,
    pub secret: Option<String>,
    pub project_id: u64,
}

impl Dsn {
    /// Parses a Dsn from a given string.
    fn from_str(dsn: &str) -> Result<Dsn> {
        let url = Url::parse(dsn)?;
        let project_id = if let Some(component_iter) = url.path_segments() {
            let components: Vec<_> = component_iter.collect();
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
            secret: url.password().map(|x| x.to_string()),
            project_id: project_id,
        })
    }

    /// Returns the URL where events should be sent.
    pub fn get_submit_url(&self) -> String {
        format!("{}://{}:{}/api/{}/store/",
                self.protocol,
                self.host,
                self.port,
                self.project_id)
    }

    /// Returns the given auth header (ts is the timestamp of the event)
    pub fn get_auth_header(&self, ts: f64) -> String {
        let mut rv = format!("Sentry \
            sentry_timestamp={}, \
            sentry_client=sentry-cli/{}, \
            sentry_version={}, \
            sentry_key={}",
                ts,
                VERSION,
                PROTOCOL_VERSION,
                self.client_id);
        if let Some(ref secret) = self.secret {
            rv = format!("{}, sentry_secret={}", rv, secret);
        }
        rv
    }
}

pub fn prepare_environment() {
    dotenv::dotenv().ok();
}

static mut CONFIG: Option<Config> = None;

/// Represents the `sentry-cli` config.
pub struct Config {
    filename: PathBuf,
    process_bound: bool,
    ini: Ini,
    cached_auth: Option<Auth>,
    cached_base_url: String,
    cached_log_level: log::LogLevelFilter,
}

impl Config {
    /// Loads the CLI config from the default location and returns it.
    pub fn from_cli_config() -> Result<Config> {
        let (filename, ini) = load_cli_config()?;
        Ok(Config {
            filename: filename,
            process_bound: false,
            cached_auth: get_default_auth(&ini),
            cached_base_url: get_default_url(&ini),
            cached_log_level: get_default_log_level(&ini)?,
            ini: ini,
        })
    }

    /// Makes this config the process bound one that can be
    /// fetched from anywhere.
    pub fn bind_to_process(mut self) -> &'static Config {
        self.process_bound = true;
        self.apply_to_process();
        unsafe {
            if CONFIG.is_some() {
                panic!("Can only bind a single config");
            }
            CONFIG = Some(self);
            CONFIG.as_ref().unwrap()
        }
    }

    /// Return the currently bound config as option.
    pub fn get_current_opt() -> Option<&'static Config> {
        unsafe {
            CONFIG.as_ref()
        }
    }

    /// Return the currently bound config.
    pub fn get_current() -> &'static Config {
        Config::get_current_opt().expect("Config not bound yet")
    }

    fn apply_to_process(&self) {
        // this can only apply to the process if we are a process config.
        if !self.process_bound { 
            return;
        }
        log::set_logger(|max_log_level| {
            max_log_level.set(self.get_log_level());
            Box::new(Logger)
        }).ok();
        if !env::var("http_proxy").is_ok() {
            if let Some(proxy) = self.get_proxy_url() {
                env::set_var("http_proxy", proxy);
            }
        }
        #[cfg(not(windows))]
        {
            use openssl_probe::init_ssl_cert_env_vars;
            init_ssl_cert_env_vars();
        }
    }

    /// Returns the config filename.
    pub fn get_filename(&self) -> &Path {
        &self.filename
    }

    /// Write the current config state back into the file.
    pub fn save(&self) -> Result<()> {
        let mut file = OpenOptions::new().write(true)
            .truncate(true)
            .create(true)
            .open(&self.filename)?;
        self.ini.write_to(&mut file)?;
        Ok(())
    }

    /// Returns the auth info
    pub fn get_auth(&self) -> Option<&Auth> {
        self.cached_auth.as_ref()
    }

    /// Updates the auth info
    pub fn set_auth(&mut self, auth: Auth) {
        self.cached_auth = Some(auth);

        self.ini.delete_from(Some("auth"), "api_key");
        self.ini.delete_from(Some("auth"), "token");
        match self.cached_auth {
            Some(Auth::Token(ref val)) => {
                self.ini.set_to(Some("auth"), "token".into(), val.to_string());
            }
            Some(Auth::Key(ref val)) => {
                self.ini.set_to(Some("auth"), "api_key".into(), val.to_string());
            }
            None => {}
        }
    }

    /// Sets the URL
    pub fn set_base_url(&mut self, url: &str) {
        self.cached_base_url = url.to_owned();
        self.ini.set_to(Some("defaults"), "url".into(), self.cached_base_url.clone());
    }

    /// Returns the base url (without trailing slashes)
    pub fn get_base_url(&self) -> Result<&str> {
        let base = self.cached_base_url.trim_right_matches('/');
        if !base.starts_with("http://") && !base.starts_with("https://") {
            fail!("bad sentry url: unknown scheme ({})", base);
        }
        if base.matches('/').count() != 2 {
            fail!("bad sentry url: not on URL root ({})", base);
        }
        Ok(base)
    }

    /// Returns the API URL for a path
    pub fn get_api_endpoint(&self, path: &str) -> Result<String> {
        let base = self.get_base_url()?;
        Ok(format!("{}/api/0/{}", base, path.trim_left_matches('/')))
    }

    /// Returns the log level.
    pub fn get_log_level(&self) -> log::LogLevelFilter {
        self.cached_log_level
    }

    /// Sets the log level.
    pub fn set_log_level(&mut self, value: log::LogLevelFilter) {
        self.cached_log_level = value;
        self.apply_to_process();
    }

    /// Indicates whether keepalive support should be enabled.  This
    /// mostly corresponds to an ini config but also has some sensible
    /// default handling.
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

    /// Returns the proxy URL if defined.
    fn get_proxy_url(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_url")
    }

    /// Returns the proxy username if defined.
    pub fn get_proxy_username(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_username")
    }

    /// Returns the proxy password if defined.
    pub fn get_proxy_password(&self) -> Option<&str> {
        self.ini.get_from(Some("http"), "proxy_password")
    }

    /// Indicates if SSL is enabled or disabled for the server.
    pub fn has_insecure_server(&self) -> bool {
        self.get_base_url().unwrap_or("").starts_with("http://")
    }

    /// Indicates whether SSL verification should be on or off.
    pub fn should_verify_ssl(&self) -> bool {
        let val = self.ini.get_from(Some("http"), "verify_ssl");
        match val {
            None => true,
            Some(val) => val == "true",
        }
    }

    /// Controls the SSL revocation check on windows.  This can be used as a
    /// workaround for misconfigured local SSL proxies.
    pub fn disable_ssl_revocation_check(&self) -> bool {
        let val = self.ini.get_from(Some("http"), "check_ssl_revoke");
        match val {
            None => true,
            Some(val) => val == "true",
        }
    }

    /// Given a match object from clap, this returns the org from it.
    pub fn get_org(&self, matches: &ArgMatches) -> Result<String> {
        Ok(matches.value_of("org")
               .map(|x| x.to_owned())
               .or_else(|| env::var("SENTRY_ORG").ok())
               .or_else(|| self.ini.get_from(Some("defaults"), "org").map(|x| x.to_owned()))
               .ok_or("An organization slug is required (provide with --org)")?)
    }

    /// Given a match object from clap, this returns a tuple in the
    /// form `(org, project)` which can either come from the match
    /// object or some defaults (envvar, ini etc.).
    pub fn get_org_and_project(&self, matches: &ArgMatches) -> Result<(String, String)> {
        let org = self.get_org(matches)?;
        let project = if let Some(project) = matches.value_of("project") {
            project.to_owned()
        } else {
            self.get_project_default()?
        };
        Ok((org, project))
    }

    /// Return the default value for a project.
    pub fn get_project_default(&self) -> Result<String> {
        Ok(env::var("SENTRY_PROJECT").ok()
            .or_else(|| self.ini.get_from(Some("defaults"), "project").map(|x| x.to_owned()))
            .ok_or("A project slug is required")?)
    }

    /// Returns the defaults for org and project.
    pub fn get_org_and_project_defaults(&self) -> (Option<String>, Option<String>) {
        (env::var("SENTRY_ORG")
             .ok()
             .or_else(|| self.ini.get_from(Some("defaults"), "org").map(|x| x.to_owned())),
         env::var("SENTRY_PROJECT")
             .ok()
             .or_else(|| self.ini.get_from(Some("defaults"), "project").map(|x| x.to_owned())))
    }

    /// Returns true if notifications should be displayed
    pub fn show_notifications(&self) -> Result<bool> {
        Ok(self.ini.get_from(Some("ui"), "show_notifications")
            .map(|x| x == "true")
            .unwrap_or(true))
    }

    /// Returns the maximum dsym upload size
    pub fn get_max_dsym_upload_size(&self) -> Result<u64> {
        Ok(self.ini.get_from(Some("dsym"), "max_upload_size")
            .and_then(|x| x.parse().ok())
            .unwrap_or(35 * 1024 * 1024))
    }

    /// Return the DSN
    pub fn get_dsn(&self) -> Result<Dsn> {
        if let Some(ref val) = env::var("SENTRY_DSN").ok() {
            Dsn::from_str(val)
        } else if let Some(val) = self.ini.get_from(Some("auth"), "dsn") {
            Dsn::from_str(val)
        } else {
            fail!("No DSN provided");
        }
    }

    /// Return device model
    pub fn get_model(&self) -> Option<String> {
        if env::var_os("DEVICE_MODEL").is_some() {
            env::var("DEVICE_MODEL").ok()
        } else if let Some(val) = self.ini.get_from(Some("device"), "model") {
            Some(String::from(val))
        } else {
            None
        }
    }

    /// Return device family
    pub fn get_family(&self) -> Option<String> {
        if env::var_os("DEVICE_FAMILY").is_some() {
            env::var("DEVICE_FAMILY").ok()
        } else if let Some(val) = self.ini.get_from(Some("device"), "family") {
            Some(String::from(val))
        } else {
            None
        }
    }

    /// Should we nag about updates?
    pub fn disable_update_nagger(&self) -> bool {
        if let Ok(var) = env::var("SENTRY_DISABLE_UPDATE_CHECK") {
            &var == "1" || &var == "true"
        } else {
            if let Some(val) = self.ini.get_from(Some("update"), "disable_check") {
                val == "true"
            } else {
                false
            }
        }
    }
}

fn find_project_config_file() -> Option<PathBuf> {
    env::current_dir().ok().and_then(|mut path| {
        loop {
            path.push(".sentryclirc");
            if path.exists() {
                return Some(path);
            }
            path.set_file_name("sentrycli.ini");
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

fn load_cli_config() -> Result<(PathBuf, Ini)> {
    let mut home_fn = env::home_dir().ok_or("Could not find home dir")?;
    home_fn.push(".sentryclirc");

    let mut rv = match fs::File::open(&home_fn) {
        Ok(mut file) => Ini::read_from(&mut file)?,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ini::new()
            } else {
                return Err(err).chain_err(
                    || "Failed to load .sentryclirc file from the home folder.");
            }
        }
    };

    let (path, mut rv) = if let Some(project_config_path) = find_project_config_file() {
        let mut f = fs::File::open(&project_config_path)
            .chain_err(|| format!("Failed to load .sentryclirc file from project path ({})",
                project_config_path.display()))?;
        let ini = Ini::read_from(&mut f)?;
        for (section, props) in ini.iter() {
            for (key, value) in props {
                rv.set_to(section.clone(), key.clone(), value.to_owned());
            }
        }
        (project_config_path, rv)
    } else {
        (home_fn, rv)
    };

    if let Ok(prop_path) = env::var("SENTRY_PROPERTIES") {
        match fs::File::open(&prop_path) {
            Ok(f) => {
                let props = match java_properties::read(f) {
                    Ok(props) => props,
                    Err(err) => {
                        return Err(Error::from(format!(
                            "Could not load java style properties file: {}", err)));
                    }
                };
                for (key, value) in props {
                    let mut iter = key.rsplitn(2, '.');
                    if let Some(key) = iter.next() {
                        let section = iter.next();
                        rv.set_to(section, key.to_string(), value);
                    }
                }
            },
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(Error::from(err)).chain_err(
                        || format!("Failed to load file referenced by SENTRY_PROPERTIES ({})",
                                   &prop_path));
                }
            }
        }
    }

    Ok((path, rv))
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {
            filename: self.filename.clone(),
            process_bound: false,
            ini: self.ini.clone(),
            cached_auth: self.cached_auth.clone(),
            cached_base_url: self.cached_base_url.clone(),
            cached_log_level: self.cached_log_level.clone(),
        }
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

fn get_default_log_level(ini: &Ini) -> Result<log::LogLevelFilter> {
    if let Ok(level_str) = env::var("SENTRY_LOG_LEVEL") {
        if let Ok(level) = level_str.parse() {
            return Ok(level);
        }
    }

    if let Some(level_str) = ini.get_from(Some("log"), "level") {
        if let Ok(level) = level_str.parse() {
            return Ok(level);
        }
    }

    Ok(log::LogLevelFilter::Warn)
}
