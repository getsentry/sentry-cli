//! This module implements config access.
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, format_err, Context, Error, Result};
use clap::ArgMatches;
use ini::Ini;
use lazy_static::lazy_static;
use log::{debug, info, set_max_level, warn};
use parking_lot::Mutex;
use secrecy::ExposeSecret;
use sentry::types::Dsn;

use crate::constants::CONFIG_INI_FILE_PATH;
use crate::constants::DEFAULT_MAX_DIF_ITEM_SIZE;
use crate::constants::DEFAULT_MAX_DIF_UPLOAD_SIZE;
use crate::constants::{CONFIG_RC_FILE_NAME, DEFAULT_RETRIES, DEFAULT_URL};
use crate::utils::auth_token::AuthToken;
use crate::utils::auth_token::AuthTokenPayload;
use crate::utils::http::is_absolute_url;

#[cfg(target_os = "macos")]
use crate::utils::xcode;

/// Represents the auth information
#[derive(Debug, Clone)]
pub enum Auth {
    Key(String),
    Token(AuthToken),
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
    cached_headers: Option<Vec<String>>,
    cached_log_level: log::LevelFilter,
    cached_vcs_remote: String,
    cached_token_data: Option<AuthTokenPayload>,
}

impl Config {
    /// Loads the CLI config from the default location and returns it.
    pub fn from_cli_config() -> Result<Config> {
        let (filename, ini) = load_cli_config()?;
        Config::from_file(filename, ini)
    }

    /// Creates Config based on provided config file.
    pub fn from_file(filename: PathBuf, ini: Ini) -> Result<Config> {
        let auth = get_default_auth(&ini);
        let token_embedded_data = match auth {
            Some(Auth::Token(ref token)) => token.payload().cloned(),
            _ => None, // get_default_auth never returns Auth::Token variant
        };

        let manually_configured_url = configured_url(&ini);
        let token_url = token_embedded_data
            .as_ref()
            .map(|td| td.url.as_str())
            .unwrap_or_default();

        let url = if token_url.is_empty() {
            manually_configured_url.unwrap_or_else(|| DEFAULT_URL.to_string())
        } else {
            warn_about_conflicting_urls(token_url, manually_configured_url.as_deref());
            token_url.into()
        };

        Ok(Config {
            filename,
            process_bound: false,
            cached_auth: auth,
            cached_base_url: url,
            cached_headers: get_default_headers(&ini),
            cached_log_level: get_default_log_level(&ini),
            cached_vcs_remote: get_default_vcs_remote(&ini),
            ini,
            cached_token_data: token_embedded_data,
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

    /// Return the global config reference.
    pub fn global() -> Result<Config> {
        let (global_filename, global_config) = load_global_config_file()?;
        Config::from_file(global_filename, global_config)
    }

    /// Makes a copy of the config in a closure and boxes it.
    pub fn make_copy<F: FnOnce(&mut Config) -> Result<()>>(&self, cb: F) -> Result<Arc<Config>> {
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
    pub fn save(&self) -> Result<()> {
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
    pub fn set_auth(&mut self, auth: Auth) -> Result<()> {
        self.cached_auth = Some(auth);

        self.ini.delete_from(Some("auth"), "api_key");
        self.ini.delete_from(Some("auth"), "token");
        match self.cached_auth {
            Some(Auth::Token(ref val)) => {
                self.cached_token_data = val.payload().cloned();

                if let Some(token_url) = self.cached_token_data.as_ref().map(|td| td.url.as_str()) {
                    self.cached_base_url = token_url.to_string();
                }

                self.ini.set_to(
                    Some("auth"),
                    "token".into(),
                    val.raw().expose_secret().clone(),
                );
            }
            Some(Auth::Key(ref val)) => {
                self.ini
                    .set_to(Some("auth"), "api_key".into(), val.to_string());
            }
            None => {}
        }

        Ok(())
    }

    /// Returns the base url (without trailing slashes)
    pub fn get_base_url(&self) -> Result<&str> {
        let base = self.cached_base_url.trim_end_matches('/');
        if !is_absolute_url(base) {
            bail!("bad sentry url: unknown scheme ({})", base);
        }
        if base.matches('/').count() != 2 {
            bail!("bad sentry url: not on URL root ({})", base);
        }
        Ok(base)
    }

    /// Sets the URL
    pub fn set_base_url(&mut self, url: &str) {
        let token_url = self
            .cached_token_data
            .as_ref()
            .map(|td| td.url.as_str())
            .unwrap_or_default();

        if !token_url.is_empty() && url != token_url {
            log::warn!(
                "Using {token_url} (embedded in token) rather than manually-configured URL {url}. \
                To use {url}, please provide an auth token for this URL."
            );
        } else {
            url.clone_into(&mut self.cached_base_url);
            self.ini
                .set_to(Some("defaults"), "url".into(), self.cached_base_url.clone());
        }
    }

    /// Sets headers that should be attached to all requests
    pub fn set_headers(&mut self, headers: Vec<String>) {
        self.cached_headers = Some(headers);
    }

    /// Get headers that should be attached to all requests
    pub fn get_headers(&self) -> Option<Vec<String>> {
        self.cached_headers.clone()
    }

    /// Returns the API URL for a path
    pub fn get_api_endpoint(&self, path: &str, base_url_override: Option<&str>) -> Result<String> {
        let base: &str = base_url_override
            .unwrap_or(self.get_base_url()?)
            .trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let path = path.trim_start_matches("api/0/");

        Ok(format!("{}/api/0/{}", base, path))
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
    pub fn get_proxy_url(&self) -> Option<String> {
        if env::var_os("http_proxy").is_some() {
            env::var("http_proxy").ok()
        } else {
            self.ini
                .get_from(Some("http"), "proxy_url")
                .map(|val| val.to_owned())
        }
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
    pub fn get_org(&self, matches: &ArgMatches) -> Result<String> {
        let org_from_token = self.cached_token_data.as_ref().map(|t| &t.org);

        let org_from_cli = matches
            .get_one::<String>("org")
            .cloned()
            .or_else(|| env::var("SENTRY_ORG").ok());

        match (org_from_token, org_from_cli) {
            (None, None) => self
                .ini
                .get_from(Some("defaults"), "org")
                .map(str::to_owned)
                .ok_or_else(|| {
                    format_err!("An organization ID or slug is required (provide with --org)")
                }),
            (None, Some(cli_org)) => Ok(cli_org),
            (Some(token_org), None) => Ok(token_org.to_string()),
            (Some(token_org), Some(cli_org)) => {
                if cli_org.is_empty() {
                    return Ok(token_org.to_owned());
                }
                if cli_org != *token_org {
                    return Err(format_err!(
                        "Two different org values supplied: `{token_org}` (from token), `{cli_org}`."
                    ));
                }
                Ok(cli_org)
            }
        }
    }

    /// Given a match object from clap, this returns the release from it.
    pub fn get_release(&self, matches: &ArgMatches) -> Result<String> {
        matches
            .get_one::<String>("release")
            .cloned()
            .or_else(|| env::var("SENTRY_RELEASE").ok())
            .ok_or_else(|| format_err!("A release slug is required (provide with --release)"))
    }

    // Backward compatibility with `releases files <VERSION>` commands.
    pub fn get_release_with_legacy_fallback(&self, matches: &ArgMatches) -> Result<String> {
        if let Some(version) = matches.get_one::<String>("version") {
            Ok(version.to_string())
        } else {
            self.get_release(matches)
        }
    }

    /// Given a match object from clap, this returns the project from it.
    pub fn get_project(&self, matches: &ArgMatches) -> Result<String> {
        self.get_projects(matches).map(|p| p[0].clone())
    }

    /// Given a match object from clap, this returns the projects from it.
    pub fn get_projects(&self, matches: &ArgMatches) -> Result<Vec<String>> {
        if let Some(projects) = matches.get_many::<String>("project") {
            Ok(projects.cloned().collect())
        } else {
            Ok(vec![self.get_project_default()?])
        }
    }

    /// Given a match object from clap, this returns a tuple in the
    /// form `(org, project)` which can either come from the match
    /// object or some defaults (envvar, ini etc.).
    pub fn get_org_and_project(&self, matches: &ArgMatches) -> Result<(String, String)> {
        let org = self.get_org(matches)?;
        let project = self.get_project(matches)?;
        Ok((org, project))
    }

    /// Return the default value for a project.
    pub fn get_project_default(&self) -> Result<String> {
        env::var("SENTRY_PROJECT")
            .ok()
            .or_else(|| {
                self.ini
                    .get_from(Some("defaults"), "project")
                    .map(str::to_owned)
            })
            .ok_or_else(|| format_err!("A project ID or slug is required (provide with --project)"))
    }

    /// Return the default pipeline env.
    pub fn get_pipeline_env(&self) -> Option<String> {
        env::var("SENTRY_PIPELINE").ok().or_else(|| {
            self.ini
                .get_from(Some("defaults"), "pipeline")
                .map(str::to_owned)
        })
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

    /// Returns true if notifications should be displayed.
    /// We only use this function in the macOS binary.
    #[cfg(target_os = "macos")]
    pub fn show_notifications(&self) -> Result<bool> {
        Ok(self
            .ini
            .get_from(Some("ui"), "show_notifications")
            .map(|x| x == "true")
            .unwrap_or(true))
    }

    /// Returns the maximum DIF upload size
    pub fn get_max_dif_archive_size(&self) -> u64 {
        let key = "max_upload_size";

        self.ini
            .get_from(Some("dif"), key)
            .or_else(|| self.ini.get_from(Some("dsym"), key))
            .and_then(|x| x.parse().ok())
            .unwrap_or(DEFAULT_MAX_DIF_UPLOAD_SIZE)
    }

    /// Returns the maximum file size of a single file inside DIF bundle
    pub fn get_max_dif_item_size(&self) -> u64 {
        let key = "max_item_size";

        self.ini
            .get_from(Some("dif"), key)
            .or_else(|| self.ini.get_from(Some("dsym"), key))
            .and_then(|x| x.parse().ok())
            .unwrap_or(DEFAULT_MAX_DIF_ITEM_SIZE)
    }

    pub fn get_max_retry_count(&self) -> Result<u32> {
        if env::var_os("SENTRY_HTTP_MAX_RETRIES").is_some() {
            Ok(env::var("SENTRY_HTTP_MAX_RETRIES")?.parse()?)
        } else if let Some(val) = self.ini.get_from(Some("http"), "max_retries") {
            Ok(val.parse()?)
        } else {
            Ok(DEFAULT_RETRIES)
        }
    }

    /// Return the DSN
    pub fn get_dsn(&self) -> Result<Dsn> {
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
        } else {
            self.ini
                .get_from(Some("defaults"), "environment")
                .map(String::from)
        }
    }

    /// Return VCS remote
    pub fn get_cached_vcs_remote(&self) -> String {
        self.cached_vcs_remote.clone()
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

    pub fn get_allow_failure(&self, matches: &ArgMatches) -> bool {
        matches.get_flag("allow_failure")
            || if let Ok(var) = env::var("SENTRY_ALLOW_FAILURE") {
                &var == "1" || &var == "true"
            } else {
                false
            }
    }
}

fn warn_about_conflicting_urls(token_url: &str, manually_configured_url: Option<&str>) {
    if let Some(manually_configured_url) = manually_configured_url {
        if manually_configured_url != token_url {
            warn!(
                "Using {token_url} (embedded in token) rather than manually-configured URL \
                {manually_configured_url}. To use {manually_configured_url}, please provide an  \
                auth token for {manually_configured_url}."
            );
        }
    }
}

fn find_global_config_file() -> Result<PathBuf> {
    let home_dir_file = dirs::home_dir().map(|p| p.join(CONFIG_RC_FILE_NAME));
    let config_dir_file = dirs::config_dir().map(|p| p.join(CONFIG_INI_FILE_PATH));
    home_dir_file
        .clone()
        .filter(|p| p.exists())
        .or(config_dir_file.filter(|p| p.exists()))
        .or(home_dir_file)
        .ok_or_else(|| format_err!("Could not find home dir"))
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

fn load_global_config_file() -> Result<(PathBuf, Ini)> {
    // Make sure to not load global configuration, as it can skew the tests results
    // during local development for different environments.
    if env::var("SENTRY_INTEGRATION_TEST").is_ok() {
        return Ok((PathBuf::new(), Ini::new()));
    }

    let filename = find_global_config_file()?;
    match fs::File::open(&filename) {
        Ok(mut file) => match Ini::read_from(&mut file) {
            Ok(ini) => Ok((filename, ini)),
            Err(err) => Err(Error::from(err).context(format!(
                "Failed to parse {CONFIG_RC_FILE_NAME} file from the home folder."
            ))),
        },
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok((filename, Ini::new()))
            } else {
                Err(Error::from(err).context(format!(
                    "Failed to load {CONFIG_RC_FILE_NAME} file from the home folder."
                )))
            }
        }
    }
}

fn failed_local_config_load_message(file_desc: &str) -> String {
    let msg = format!("Failed to load {file_desc}.");
    #[cfg(target_os = "macos")]
    if xcode::launched_from_xcode() {
        return msg + (" Hint: Please ensure that ${SRCROOT}/.sentryclirc is added to the Input Files of this Xcode Build Phases script.");
    }
    msg
}

fn load_cli_config() -> Result<(PathBuf, Ini)> {
    let (global_filename, mut rv) = load_global_config_file()?;

    let (path, mut rv) = if let Some(project_config_path) = find_project_config_file() {
        let file_desc = format!(
            "{} file from project path ({})",
            CONFIG_RC_FILE_NAME,
            project_config_path.display()
        );
        let mut f = fs::File::open(&project_config_path)
            .context(failed_local_config_load_message(&file_desc))?;
        let ini = Ini::read_from(&mut f).context(format!("Failed to parse {file_desc}"))?;
        for (section, props) in ini.iter() {
            for (key, value) in props.iter() {
                rv.set_to(section, key.to_string(), value.to_owned());
            }
        }
        (project_config_path, rv)
    } else {
        (global_filename, rv)
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
                info!(
                    "Loaded file referenced by SENTRY_PROPERTIES ({})",
                    &prop_path
                );
                for (key, value) in props {
                    let mut iter = key.rsplitn(2, '.');
                    if let Some(key) = iter.next() {
                        let section = iter.next();
                        rv.set_to(section, key.to_string(), value);
                    } else {
                        debug!("Incorrect properties file key: {}", key);
                    }
                }
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(Error::from(err).context(format!(
                        "Failed to load file referenced by SENTRY_PROPERTIES ({})",
                        &prop_path
                    )));
                } else {
                    warn!(
                        "Failed to find file referenced by SENTRY_PROPERTIES ({})",
                        &prop_path
                    );
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
            cached_headers: self.cached_headers.clone(),
            cached_log_level: self.cached_log_level,
            cached_vcs_remote: self.cached_vcs_remote.clone(),
            cached_token_data: self.cached_token_data.clone(),
        }
    }
}

#[allow(clippy::manual_map)]
fn get_default_auth(ini: &Ini) -> Option<Auth> {
    if let Ok(val) = env::var("SENTRY_AUTH_TOKEN") {
        Some(Auth::Token(val.into()))
    } else if let Ok(val) = env::var("SENTRY_API_KEY") {
        Some(Auth::Key(val))
    } else if let Some(val) = ini.get_from(Some("auth"), "token") {
        Some(Auth::Token(val.into()))
    } else if let Some(val) = ini.get_from(Some("auth"), "api_key") {
        Some(Auth::Key(val.to_owned()))
    } else {
        None
    }
}

/// Returns the URL configured in the SENTRY_URL environment variable or provided ini (in that
/// order of precedence), or returns None if neither is set.
fn configured_url(ini: &Ini) -> Option<String> {
    env::var("SENTRY_URL").ok().or_else(|| {
        ini.get_from(Some("defaults"), "url")
            .map(|url| url.to_owned())
    })
}

fn get_default_headers(ini: &Ini) -> Option<Vec<String>> {
    if let Ok(val) = env::var("CUSTOM_HEADER") {
        Some(vec![val])
    } else {
        ini.get_from(Some("defaults"), "custom_header")
            .map(|val| vec![val.to_owned()])
    }
}

fn get_default_log_level(ini: &Ini) -> log::LevelFilter {
    if let Ok(level_str) = env::var("SENTRY_LOG_LEVEL") {
        if let Ok(level) = level_str.parse() {
            return level;
        }
    }

    if let Some(level_str) = ini.get_from(Some("log"), "level") {
        if let Ok(level) = level_str.parse() {
            return level;
        }
    }

    log::LevelFilter::Warn
}

/// Get the default VCS remote.
///
/// To be backward compatible the default remote is still
/// origin.
fn get_default_vcs_remote(ini: &Ini) -> String {
    if let Ok(remote) = env::var("SENTRY_VCS_REMOTE") {
        remote
    } else if let Some(remote) = ini.get_from(Some("defaults"), "vcs_remote") {
        remote.to_string()
    } else {
        "origin".to_string()
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use super::*;

    #[test]
    fn test_get_api_endpoint() {
        let config = Config {
            filename: PathBuf::from("/path/to/config"),
            process_bound: false,
            ini: Default::default(),
            cached_auth: None,
            cached_base_url: "https://sentry.io/".to_string(),
            cached_headers: None,
            cached_log_level: LevelFilter::Off,
            cached_vcs_remote: String::new(),
            cached_token_data: None,
        };

        assert_eq!(
            config
                .get_api_endpoint("/organizations/test-org/chunk-upload/", None)
                .unwrap(),
            "https://sentry.io/api/0/organizations/test-org/chunk-upload/"
        );

        assert_eq!(
            config
                .get_api_endpoint("/api/0/organizations/test-org/chunk-upload/", None)
                .unwrap(),
            "https://sentry.io/api/0/organizations/test-org/chunk-upload/"
        );

        assert_eq!(
            config
                .get_api_endpoint(
                    "/api/0/organizations/test-org/chunk-upload/",
                    Some("https://us.sentry.io/")
                )
                .unwrap(),
            "https://us.sentry.io/api/0/organizations/test-org/chunk-upload/"
        );
    }
}
