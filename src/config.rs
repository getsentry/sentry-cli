//! This module implements config access.
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::ArgMatches;
use failure::{bail, err_msg, Error, ResultExt};
use ini::Ini;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use sentry::internals::Dsn;

use crate::constants::{CONFIG_RC_FILE_NAME, DEFAULT_RETRIES, DEFAULT_URL};
use crate::utils::logging::set_max_level;

/// Represents the auth information
#[derive(Debug, Clone)]
pub enum Auth {
    Key(String),
    Token(String),
}

pub fn prepare_environment() {
    dotenv::dotenv().ok();
}

lazy_static! {
    static ref CONFIG: Mutex<Option<Arc<Config>>> = Mutex::new(None);
}

/// Represents the `sentry-cli` config.
pub struct Config {
    filename: PathBuf,
    process_bound: bool,
    ini: Ini,
    cached_auth: Option<Auth>,
    cached_base_url: String,
    cached_log_level: log::LevelFilter,
}

impl Config {
    /// Loads the CLI config from the default location and returns it.
    pub fn from_cli_config() -> Result<Config, Error> {
        let (filename, ini) = load_cli_config()?;
        Ok(Config {
            filename,
            process_bound: false,
            cached_auth: get_default_auth(&ini),
            cached_base_url: get_default_url(&ini),
            cached_log_level: get_default_log_level(&ini)?,
            ini,
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
        Config::current()
    }

    /// Return the currently bound config as option.
    pub fn current_opt() -> Option<Arc<Config>> {
        CONFIG.lock().as_ref().cloned()
    }

    /// Return the currently bound config.
    pub fn current() -> Arc<Config> {
        Config::current_opt().expect("Config not bound yet")
    }

    /// Makes a copy of the config in a closure and boxes it.
    pub fn make_copy<F: FnOnce(&mut Config) -> Result<(), Error>>(
        &self,
        cb: F,
    ) -> Result<Arc<Config>, Error> {
        let mut new_config = self.clone();
        cb(&mut new_config)?;
        Ok(Arc::new(new_config))
    }

    fn apply_to_process(&self) {
        // this can only apply to the process if we are a process config.
        if !self.process_bound {
            return;
        }
        set_max_level(self.get_log_level());
        #[cfg(feature = "with_crash_reporting")]
        {
            crate::utils::crashreporting::bind_configured_client(Some(self));
        }
        if env::var("http_proxy").is_err() {
            if let Some(proxy) = self.get_proxy_url() {
                env::set_var("http_proxy", proxy);
            }
        }
        #[cfg(not(windows))]
        {
            openssl_probe::init_ssl_cert_env_vars();
        }
    }

    /// Returns the config filename.
    pub fn get_filename(&self) -> &Path {
        &self.filename
    }

    /// Write the current config state back into the file.
    pub fn save(&self) -> Result<(), Error> {
        let mut options = OpenOptions::new();
        options.write(true).truncate(true).create(true);

        // Remove all non-user permissions for the newly created file
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options.open(&self.filename)?;
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
                self.ini
                    .set_to(Some("auth"), "token".into(), val.to_string());
            }
            Some(Auth::Key(ref val)) => {
                self.ini
                    .set_to(Some("auth"), "api_key".into(), val.to_string());
            }
            None => {}
        }
    }

    /// Sets the URL
    pub fn set_base_url(&mut self, url: &str) {
        self.cached_base_url = url.to_owned();
        self.ini
            .set_to(Some("defaults"), "url".into(), self.cached_base_url.clone());
    }

    /// Returns the base url (without trailing slashes)
    pub fn get_base_url(&self) -> Result<&str, Error> {
        let base = self.cached_base_url.trim_end_matches('/');
        if !base.starts_with("http://") && !base.starts_with("https://") {
            bail!("bad sentry url: unknown scheme ({})", base);
        }
        if base.matches('/').count() != 2 {
            bail!("bad sentry url: not on URL root ({})", base);
        }
        Ok(base)
    }

    /// Returns the API URL for a path
    pub fn get_api_endpoint(&self, path: &str) -> Result<String, Error> {
        let base = self.get_base_url()?;
        Ok(format!("{}/api/0/{}", base, path.trim_start_matches('/')))
    }

    /// Returns the log level.
    pub fn get_log_level(&self) -> log::LevelFilter {
        self.cached_log_level
    }

    /// Sets the log level.
    pub fn set_log_level(&mut self, value: log::LevelFilter) {
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

    /// Indicates whether uploads may use gzip transfer encoding.
    pub fn allow_transfer_encoding(&self) -> bool {
        let val = self.ini.get_from(Some("http"), "transfer_encoding");
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
    pub fn get_org(&self, matches: &ArgMatches<'_>) -> Result<String, Error> {
        Ok(matches
            .value_of("org")
            .map(str::to_owned)
            .or_else(|| env::var("SENTRY_ORG").ok())
            .or_else(|| {
                self.ini
                    .get_from(Some("defaults"), "org")
                    .map(str::to_owned)
            })
            .ok_or_else(|| err_msg("An organization slug is required (provide with --org)"))?)
    }

    /// Given a match object from clap, this returns a tuple in the
    /// form `(org, project)` which can either come from the match
    /// object or some defaults (envvar, ini etc.).
    pub fn get_org_and_project(&self, matches: &ArgMatches<'_>) -> Result<(String, String), Error> {
        let org = self.get_org(matches)?;
        let project = if let Some(project) = matches.value_of("project") {
            project.to_owned()
        } else {
            self.get_project_default()?
        };
        Ok((org, project))
    }

    /// Return the default value for a project.
    pub fn get_project_default(&self) -> Result<String, Error> {
        Ok(env::var("SENTRY_PROJECT")
            .ok()
            .or_else(|| {
                self.ini
                    .get_from(Some("defaults"), "project")
                    .map(str::to_owned)
            })
            .ok_or_else(|| err_msg("A project slug is required"))?)
    }

    /// Returns the defaults for org and project.
    pub fn get_org_and_project_defaults(&self) -> (Option<String>, Option<String>) {
        (
            env::var("SENTRY_ORG").ok().or_else(|| {
                self.ini
                    .get_from(Some("defaults"), "org")
                    .map(str::to_owned)
            }),
            env::var("SENTRY_PROJECT").ok().or_else(|| {
                self.ini
                    .get_from(Some("defaults"), "project")
                    .map(str::to_owned)
            }),
        )
    }

    /// Returns true if notifications should be displayed
    pub fn show_notifications(&self) -> Result<bool, Error> {
        Ok(self
            .ini
            .get_from(Some("ui"), "show_notifications")
            .map(|x| x == "true")
            .unwrap_or(true))
    }

    /// Returns the maximum DIF upload size
    pub fn get_max_dif_archive_size(&self) -> Result<u64, Error> {
        Ok(self
            .ini
            .get_from(Some("dsym"), "max_upload_size")
            .and_then(|x| x.parse().ok())
            .unwrap_or(35 * 1024 * 1024))
    }

    pub fn get_max_retry_count(&self) -> Result<u32, Error> {
        if env::var_os("SENTRY_HTTP_MAX_RETRIES").is_some() {
            Ok(env::var("SENTRY_HTTP_MAX_RETRIES")?.parse()?)
        } else if let Some(val) = self.ini.get_from(Some("http"), "max_retries") {
            Ok(val.parse()?)
        } else {
            Ok(DEFAULT_RETRIES)
        }
    }

    /// Return the DSN
    pub fn get_dsn(&self) -> Result<Dsn, Error> {
        if let Ok(val) = env::var("SENTRY_DSN") {
            Ok(val.parse()?)
        } else if let Some(val) = self.ini.get_from(Some("auth"), "dsn") {
            Ok(val.parse()?)
        } else {
            bail!("No DSN provided");
        }
    }

    /// Return the environment
    pub fn get_environment(&self) -> Option<String> {
        if env::var_os("SENTRY_ENVIRONMENT").is_some() {
            env::var("SENTRY_ENVIRONMENT").ok()
        } else if let Some(val) = self.ini.get_from(Some("defaults"), "environment") {
            Some(String::from(val))
        } else {
            None
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
        } else if let Some(val) = self.ini.get_from(Some("update"), "disable_check") {
            val == "true"
        } else {
            false
        }
    }

    /// Does this installation want errors to sentry?
    pub fn internal_sentry_dsn(&self) -> Option<Dsn> {
        if !self
            .ini
            .get_from(Some("crash_reporting"), "enabled")
            .map(|x| x == "1" || x == "true")
            .unwrap_or(false)
        {
            return None;
        }
        if let Some(val) = self.ini.get_from(Some("crash_reporting"), "sentry_dsn") {
            val.parse().ok()
        } else {
            None
        }
    }
}

fn find_project_config_file() -> Option<PathBuf> {
    env::current_dir().ok().and_then(|mut path| loop {
        path.push(CONFIG_RC_FILE_NAME);
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
    })
}

fn load_cli_config() -> Result<(PathBuf, Ini), Error> {
    let mut home_fn = dirs::home_dir().ok_or_else(|| err_msg("Could not find home dir"))?;
    home_fn.push(CONFIG_RC_FILE_NAME);

    let mut rv = match fs::File::open(&home_fn) {
        Ok(mut file) => Ini::read_from(&mut file)?,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ini::new()
            } else {
                return Err(Error::from(err)
                    .context("Failed to load .sentryclirc file from the home folder.")
                    .into());
            }
        }
    };

    let (path, mut rv) = if let Some(project_config_path) = find_project_config_file() {
        let mut f = fs::File::open(&project_config_path).with_context(|_| {
            format!(
                "Failed to load {} file from project path ({})",
                CONFIG_RC_FILE_NAME,
                project_config_path.display()
            )
        })?;
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
                        bail!("Could not load java style properties file: {}", err);
                    }
                };
                for (key, value) in props {
                    let mut iter = key.rsplitn(2, '.');
                    if let Some(key) = iter.next() {
                        let section = iter.next();
                        rv.set_to(section, key.to_string(), value);
                    }
                }
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(Error::from(err)
                        .context(format!(
                            "Failed to load file referenced by SENTRY_PROPERTIES ({})",
                            &prop_path
                        ))
                        .into());
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
            cached_log_level: self.cached_log_level,
        }
    }
}

fn get_default_auth(ini: &Ini) -> Option<Auth> {
    if let Ok(val) = env::var("SENTRY_AUTH_TOKEN") {
        Some(Auth::Token(val))
    } else if let Ok(val) = env::var("SENTRY_API_KEY") {
        Some(Auth::Key(val))
    } else if let Some(val) = ini.get_from(Some("auth"), "token") {
        Some(Auth::Token(val.to_owned()))
    } else if let Some(val) = ini.get_from(Some("auth"), "api_key") {
        Some(Auth::Key(val.to_owned()))
    } else {
        None
    }
}

fn get_default_url(ini: &Ini) -> String {
    if let Ok(val) = env::var("SENTRY_URL") {
        val
    } else if let Some(val) = ini.get_from(Some("defaults"), "url") {
        val.to_owned()
    } else {
        DEFAULT_URL.to_owned()
    }
}

fn get_default_log_level(ini: &Ini) -> Result<log::LevelFilter, Error> {
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

    Ok(log::LevelFilter::Warn)
}
