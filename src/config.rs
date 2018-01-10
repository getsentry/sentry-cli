//! This module implements config access.
use std::io;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dotenv;
use log;
use clap::ArgMatches;
use url::Url;
use parking_lot::Mutex;

use prelude::*;
use constants::{DEFAULT_URL, VERSION, PROTOCOL_VERSION};
use utils::{Logger, RcFile};

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

lazy_static! {
    static ref CONFIG: Mutex<Option<Arc<Config>>> = Mutex::new(None);
}

/// Represents the `sentry-cli` config.
pub struct Config {
    process_bound: bool,
    rcfile: RcFile,
    cached_auth: Option<Auth>,
    cached_base_url: String,
    cached_log_level: log::LogLevelFilter,
}

impl Config {
    /// Loads the CLI config from the default location and returns it.
    pub fn from_cli_config() -> Result<Config> {
        let rcfile = load_cli_config()?;
        Ok(Config {
            process_bound: false,
            cached_auth: get_default_auth(&rcfile),
            cached_base_url: get_default_url(&rcfile),
            cached_log_level: get_default_log_level(&rcfile)?,
            rcfile: rcfile,
        })
    }

    /// Makes this config the process bound one that can be
    /// fetched from anywhere.
    pub fn bind_to_process(mut self) -> Arc<Config> {
        self.process_bound = true;
        self.apply_to_process();
        {
            let mut cfg = CONFIG.lock();
            *cfg = Some(Arc::new(self));
        }
        Config::get_current()
    }

    /// Return the currently bound config as option.
    pub fn get_current_opt() -> Option<Arc<Config>> {
        CONFIG.lock().as_ref().map(|x| x.clone())
    }

    /// Return the currently bound config.
    pub fn get_current() -> Arc<Config> {
        Config::get_current_opt().expect("Config not bound yet")
    }

    /// Makes a copy of the config in a closure and boxes it.
    pub fn make_copy<F: FnOnce(&mut Config) -> Result<()>>(&self, cb: F)
        -> Result<Arc<Config>>
    {
        let mut new_config = self.clone();
        cb(&mut new_config)?;
        Ok(Arc::new(new_config))
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
        self.rcfile.filename().unwrap()
    }

    /// Write the current config state back into the file.
    pub fn save(&self) -> Result<()> {
        self.rcfile.save()
    }

    /// Returns the auth info
    pub fn get_auth(&self) -> Option<&Auth> {
        self.cached_auth.as_ref()
    }

    /// Updates the auth info
    pub fn set_auth(&mut self, auth: Auth) {
        self.cached_auth = Some(auth);

        self.rcfile.remove("auth.api_key");
        self.rcfile.remove("auth.token");
        match self.cached_auth {
            Some(Auth::Token(ref val)) => {
                self.rcfile.set("auth.token", val);
            }
            Some(Auth::Key(ref val)) => {
                self.rcfile.set("auth.api_key", val);
            }
            None => {}
        }
    }

    /// Sets the URL
    pub fn set_base_url(&mut self, url: &str) {
        self.cached_base_url = url.to_owned();
        self.rcfile.set("defaults.url", &self.cached_base_url);
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
        let val = self.rcfile.get("http.keepalive");
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
        self.rcfile.get("http.proxy_url")
    }

    /// Returns the proxy username if defined.
    pub fn get_proxy_username(&self) -> Option<&str> {
        self.rcfile.get("http.proxy_username")
    }

    /// Returns the proxy password if defined.
    pub fn get_proxy_password(&self) -> Option<&str> {
        self.rcfile.get("http.proxy_password")
    }

    /// Indicates if SSL is enabled or disabled for the server.
    pub fn has_insecure_server(&self) -> bool {
        self.get_base_url().unwrap_or("").starts_with("http://")
    }

    /// Indicates whether SSL verification should be on or off.
    pub fn should_verify_ssl(&self) -> bool {
        let val = self.rcfile.get("http.verify_ssl");
        match val {
            None => true,
            Some(val) => val == "true",
        }
    }

    /// Controls the SSL revocation check on windows.  This can be used as a
    /// workaround for misconfigured local SSL proxies.
    pub fn disable_ssl_revocation_check(&self) -> bool {
        let val = self.rcfile.get("http.check_ssl_revoke");
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
               .or_else(|| self.rcfile.get("defaults.org").map(|x| x.to_owned()))
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
            .or_else(|| self.rcfile.get("defaults.project").map(|x| x.to_owned()))
            .ok_or("A project slug is required")?)
    }

    /// Returns the defaults for org and project.
    pub fn get_org_and_project_defaults(&self) -> (Option<String>, Option<String>) {
        (env::var("SENTRY_ORG")
             .ok()
             .or_else(|| self.rcfile.get("defaults.org").map(|x| x.to_owned())),
         env::var("SENTRY_PROJECT")
             .ok()
             .or_else(|| self.rcfile.get("defaults.project").map(|x| x.to_owned())))
    }

    /// Returns true if notifications should be displayed
    pub fn show_notifications(&self) -> Result<bool> {
        Ok(self.rcfile.get("ui.show_notifications")
            .map(|x| x == "true")
            .unwrap_or(true))
    }

    /// Returns the maximum dsym upload size
    pub fn get_max_dsym_upload_size(&self) -> Result<u64> {
        Ok(self.rcfile.get("dsym.max_upload_size")
            .and_then(|x| x.parse().ok())
            .unwrap_or(35 * 1024 * 1024))
    }

    /// Return the DSN
    pub fn get_dsn(&self) -> Result<Dsn> {
        if let Some(ref val) = env::var("SENTRY_DSN").ok() {
            Dsn::from_str(val)
        } else if let Some(val) = self.rcfile.get("auth.dsn") {
            Dsn::from_str(val)
        } else {
            fail!("No DSN provided");
        }
    }

    /// Return device model
    pub fn get_model(&self) -> Option<String> {
        if env::var_os("DEVICE_MODEL").is_some() {
            env::var("DEVICE_MODEL").ok()
        } else if let Some(val) = self.rcfile.get("device.model") {
            Some(String::from(val))
        } else {
            None
        }
    }

    /// Return device family
    pub fn get_family(&self) -> Option<String> {
        if env::var_os("DEVICE_FAMILY").is_some() {
            env::var("DEVICE_FAMILY").ok()
        } else if let Some(val) = self.rcfile.get("device.family") {
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
            if let Some(val) = self.rcfile.get("update.disable_check") {
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
            path.set_file_name("sentry.properties");
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

fn load_cli_config() -> Result<RcFile> {
    let mut home_fn = env::home_dir().ok_or("Could not find home dir")?;
    home_fn.push(".sentryclirc");

    let mut rv = match fs::File::open(&home_fn) {
        Ok(mut file) => RcFile::open(&mut file)?,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                RcFile::new()
            } else {
                return Err(err).chain_err(
                    || "Failed to load .sentryclirc file from the home folder.");
            }
        }
    };

    let path = if let Some(project_config_path) = find_project_config_file() {
        let project_conf = RcFile::open_path(&project_config_path)
            .chain_err(|| format!("Failed to load .sentryclirc file from project path ({})",
                project_config_path.display()))?;
        rv.update(&project_conf);
        project_config_path
    } else {
        home_fn
    };
    rv.set_filename(Some(&path));

    if let Ok(prop_path) = env::var("SENTRY_PROPERTIES") {
        match fs::File::open(&prop_path) {
            Ok(f) => {
                let prop_conf = RcFile::open(f)
                    .chain_err(|| format!("Failed to load java style properties file ({})",
                        prop_path))?;
                rv.update(&prop_conf);
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

    println!("{:#?}", &rv);
    Ok(rv)
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {
            process_bound: false,
            rcfile: self.rcfile.clone(),
            cached_auth: self.cached_auth.clone(),
            cached_base_url: self.cached_base_url.clone(),
            cached_log_level: self.cached_log_level.clone(),
        }
    }
}

fn get_default_auth(rcfile: &RcFile) -> Option<Auth> {
    if let Some(ref val) = env::var("SENTRY_AUTH_TOKEN").ok() {
        Some(Auth::Token(val.to_owned()))
    } else if let Some(ref val) = env::var("SENTRY_API_KEY").ok() {
        Some(Auth::Key(val.to_owned()))
    } else if let Some(val) = rcfile.get("auth.token") {
        Some(Auth::Token(val.to_owned()))
    } else if let Some(val) = rcfile.get("auth.api_key") {
        Some(Auth::Key(val.to_owned()))
    } else {
        None
    }
}

fn get_default_url(rcfile: &RcFile) -> String {
    if let Some(ref val) = env::var("SENTRY_URL").ok() {
        val.to_owned()
    } else if let Some(val) = rcfile.get("defaults.url") {
        val.to_owned()
    } else {
        DEFAULT_URL.to_owned()
    }
}

fn get_default_log_level(rcfile: &RcFile) -> Result<log::LogLevelFilter> {
    if let Ok(level_str) = env::var("SENTRY_LOG_LEVEL") {
        if let Ok(level) = level_str.parse() {
            return Ok(level);
        }
    }

    if let Some(level_str) = rcfile.get("log.level") {
        if let Ok(level) = level_str.parse() {
            return Ok(level);
        }
    }

    Ok(log::LogLevelFilter::Warn)
}
