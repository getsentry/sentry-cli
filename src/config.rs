//! This module implements config access.
use std::env;
use std::env::VarError;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write as _};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, format_err, Context as _, Error, Result};
use clap::ArgMatches;
use ini::Ini;
use lazy_static::lazy_static;
use log::{debug, info, set_max_level, warn};
use parking_lot::Mutex;
use secrecy::ExposeSecret as _;
use sentry::types::Dsn;
use uuid::Uuid;

use crate::constants::CONFIG_INI_FILE_PATH;
use crate::constants::DEFAULT_MAX_DIF_ITEM_SIZE;
use crate::constants::{CONFIG_RC_FILE_NAME, DEFAULT_RETRIES, DEFAULT_URL};
use crate::utils::args;
use crate::utils::auth_token::AuthToken;
use crate::utils::auth_token::AuthTokenPayload;
use crate::utils::http::is_absolute_url;

use crate::utils::non_empty::NonEmptyVec;
#[cfg(target_os = "macos")]
use crate::utils::xcode;

const MAX_RETRIES_ENV_VAR: &str = "SENTRY_HTTP_MAX_RETRIES";
const MAX_RETRIES_INI_KEY: &str = "max_retries";

/// Represents the auth information
#[derive(Debug, Clone)]
pub enum Auth {
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
    max_retries: u32,
}

impl Config {
    /// Loads config files and applies URL and token values from the environment and CLI.
    pub fn from_cli_config(cli_url: Option<&str>, cli_token: Option<&AuthToken>) -> Result<Config> {
        let (global_filename, mut rv) = load_global_config_file()?;
        let mut warning = None;

        let (path, mut rv) = if let Some(project_config_path) = find_project_config_file() {
            let file_desc = format!(
                "{CONFIG_RC_FILE_NAME} file from project path ({})",
                project_config_path.display()
            );
            let mut f = fs::File::open(&project_config_path)
                .context(failed_local_config_load_message(&file_desc))?;
            let ini = Ini::read_from(&mut f).context(format!("Failed to parse {file_desc}"))?;
            warning = merge_config_source(&mut rv, &ini);
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
                            bail!("Could not load java style properties file: {err}");
                        }
                    };
                    info!(
                        "Loaded file referenced by SENTRY_PROPERTIES ({})",
                        &prop_path
                    );
                    let mut properties_ini = Ini::new();
                    for (key, value) in props {
                        let mut iter = key.rsplitn(2, '.');
                        if let Some(key) = iter.next() {
                            let section = iter.next();
                            properties_ini.set_to(section, key.to_owned(), value);
                        } else {
                            debug!("Incorrect properties file key: {key}");
                        }
                    }
                    warning = merge_config_source(&mut rv, &properties_ini).or(warning);
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

        let mut auth_and_url = AuthAndUrl::from_ini(&rv);
        let runtime_warning = auth_and_url.merge_runtime(
            env::var("SENTRY_URL").ok(),
            env::var("SENTRY_AUTH_TOKEN").ok().map(AuthToken::from),
            cli_url,
            cli_token,
        );
        if let Some(warning) = runtime_warning.or(warning) {
            warn!("{warning}");
        }

        Ok(Config::from_file_and_auth(path, rv, auth_and_url))
    }

    /// Creates a config without applying runtime URL or auth token overrides.
    ///
    /// Other environment-backed settings retain their existing behavior.
    pub fn from_file(filename: PathBuf, ini: Ini) -> Self {
        let auth_and_url = AuthAndUrl::from_ini(&ini);
        Self::from_file_and_auth(filename, ini, auth_and_url)
    }

    /// Constructs a config that persists `ini` while using the separately selected auth and URL.
    fn from_file_and_auth(filename: PathBuf, ini: Ini, auth_and_url: AuthAndUrl) -> Self {
        let auth = auth_and_url.token.map(Auth::Token);
        let token_data = match auth {
            Some(Auth::Token(ref token)) => token.payload().cloned(),
            _ => None,
        };
        let token_url = token_data
            .as_ref()
            .map(|data| data.url.as_str())
            .unwrap_or_default();
        let base_url = if token_url.is_empty() {
            auth_and_url.url.unwrap_or_else(|| DEFAULT_URL.to_owned())
        } else {
            warn_about_conflicting_urls(token_url, auth_and_url.url.as_deref());
            token_url.to_owned()
        };

        Config {
            filename,
            process_bound: false,
            cached_auth: auth,
            cached_base_url: base_url,
            cached_headers: get_default_headers(&ini),
            cached_log_level: get_default_log_level(&ini),
            cached_vcs_remote: get_default_vcs_remote(&ini),
            max_retries: get_max_retries(&ini),
            ini,
            cached_token_data: token_data,
        }
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
        Ok(Config::from_file(global_filename, global_config))
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
        #[expect(deprecated)]
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
        // Make a unique temp path, containing a random UUID
        let temp_path = self
            .filename
            .clone()
            .with_added_extension(Uuid::new_v4().to_string());

        let mut options = OpenOptions::new();

        // Set options so that the file fails to be written if it already exists. It should not
        // exist, because the path contains a random UUID.
        options.write(true).create_new(true);

        // Remove all non-user permissions for the newly created file
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }

        {
            let mut file = options.open(&temp_path)?;
            self.ini.write_to(&mut file).and_then(|()| file.flush())
            // drop file handle
        }
        .and_then(|()| fs::rename(&temp_path, &self.filename))
        .inspect_err(|_| {
            // Best-effort cleanup attempt of the temporary file; errors intentionally ignored.
            let _ = fs::remove_file(&temp_path);
        })
        .map_err(Into::into)
    }

    /// Returns the auth info selected for the current process.
    pub fn get_auth(&self) -> Option<&Auth> {
        self.cached_auth.as_ref()
    }

    /// Returns the auth info that would be persisted when this config is saved.
    pub fn get_persisted_auth(&self) -> Option<Auth> {
        self.ini
            .get_from(Some("auth"), "token")
            .map(|token| Auth::Token(token.into()))
    }

    /// Updates the auth info
    pub fn set_auth(&mut self, auth: Auth) {
        self.cached_auth = Some(auth);

        self.ini.delete_from(Some("auth"), "token");
        match self.cached_auth {
            Some(Auth::Token(ref val)) => {
                self.cached_token_data = val.payload().cloned();

                if let Some(token_url) = self.cached_token_data.as_ref().map(|td| td.url.as_str()) {
                    self.cached_base_url = token_url.to_owned();
                }

                self.ini.set_to(
                    Some("auth"),
                    "token".into(),
                    val.raw().expose_secret().clone(),
                );
            }
            None => {}
        }
    }

    /// Updates the auth token and URL as a pair.
    pub fn set_auth_and_url(&mut self, auth: Auth, url: &str) {
        self.set_auth(auth);
        url.clone_into(&mut self.cached_base_url);
        self.ini
            .set_to(Some("defaults"), "url".into(), url.to_owned());
    }

    /// Returns the base url (without trailing slashes)
    pub fn get_base_url(&self) -> Result<&str> {
        let base = self.cached_base_url.trim_end_matches('/');
        if !is_absolute_url(base) {
            bail!("bad sentry url: unknown scheme ({base})");
        }
        if base.matches('/').count() != 2 {
            bail!("bad sentry url: not on URL root ({base})");
        }
        Ok(base)
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

        Ok(format!("{base}/api/0/{path}"))
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
    ///
    /// Parses the `verify_ssl` key fron the `http` section. Returns `false` only when the key
    /// equals `"false"` on a case-insensitive basis; returns `true` otherwise.
    pub fn should_verify_ssl(&self) -> bool {
        self.ini
            .get_from(Some("http"), "verify_ssl")
            .map(|val| !val.eq_ignore_ascii_case("false"))
            .unwrap_or(true)
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
                    format_err!("An organization ID or slug is required (provide with --org, set SENTRY_ORG, or use an org-scoped auth token)")
                }),
            (None, Some(cli_org)) => Ok(cli_org),
            (Some(token_org), None) => Ok(token_org.clone()),
            (Some(token_org), Some(cli_org)) => {
                if cli_org != *token_org {
                    log::warn!(
                        "Using organization `{token_org}` (embedded in token) rather \
                        than manually-configured organization `{cli_org}`. To use \
                        `{cli_org}`, please provide an auth token for this organization."
                    );
                }
                Ok(token_org.into())
            }
        }
    }

    /// Given a match object from clap, this returns the release from it.
    pub fn get_release(&self, matches: &ArgMatches) -> Result<String> {
        matches
            .get_one::<String>("release")
            .cloned()
            .or_else(|| {
                env::var("SENTRY_RELEASE").ok().filter(|v| {
                    !v.is_empty()
                        && args::validate_release(v)
                            .inspect_err(|e| {
                                warn!("Ignoring invalid SENTRY_RELEASE environment variable: {e}")
                            })
                            .is_ok()
                })
            })
            .ok_or_else(|| {
                format_err!(
                    "A release slug is required (provide with --release or by \
                    setting the SENTRY_RELEASE environment variable)"
                )
            })
    }

    // Backward compatibility with `releases files <VERSION>` commands.
    pub fn get_release_with_legacy_fallback(&self, matches: &ArgMatches) -> Result<String> {
        if let Some(version) = matches.get_one::<String>("version") {
            Ok(version.clone())
        } else {
            self.get_release(matches)
        }
    }

    /// Given a match object from clap, this returns the project from it.
    pub fn get_project(&self, matches: &ArgMatches) -> Result<String> {
        self.get_projects(matches).map(|p| p[0].clone())
    }

    /// Given a match object from clap, this returns the projects from it.
    pub fn get_projects(&self, matches: &ArgMatches) -> Result<NonEmptyVec<String>> {
        Ok(match matches.get_many::<String>("project") {
            Some(projects) => projects
                .cloned()
                .collect::<Vec<_>>()
                .try_into()
                .expect("if matches.get_many() is Some, the returned iterator is non-empty"),
            None => [self.get_project_default()?].into(),
        })
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

    /// Returns the maximum file size of a single file inside DIF bundle
    pub fn get_max_dif_item_size(&self) -> u64 {
        let key = "max_item_size";

        self.ini
            .get_from(Some("dif"), key)
            .or_else(|| self.ini.get_from(Some("dsym"), key))
            .and_then(|x| x.parse().ok())
            .unwrap_or(DEFAULT_MAX_DIF_ITEM_SIZE)
    }

    /// Returns the configured maximum number of retries for failed HTTP requests.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
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

/// Obtains the maximum number of retries from the environment or the ini file.
/// Environment variable takes precedence over the ini file. If neither is set,
/// the default value is returned.
fn get_max_retries(ini: &Ini) -> u32 {
    match max_retries_from_env() {
        Ok(Some(val)) => return val,
        Ok(None) => (),
        Err(e) => {
            warn!("Ignoring invalid {MAX_RETRIES_ENV_VAR} environment variable: {e}");
        }
    };

    match max_retries_from_ini(ini) {
        Ok(Some(val)) => return val,
        Ok(None) => (),
        Err(e) => {
            warn!("Ignoring invalid {MAX_RETRIES_INI_KEY} ini key: {e}");
        }
    };

    DEFAULT_RETRIES
}

/// Computes the maximum number of retries from the `SENTRY_HTTP_MAX_RETRIES` environment variable.
/// Returns `Ok(None)` if the environment variable is not set, other errors are returned as is.
fn max_retries_from_env() -> Result<Option<u32>> {
    match env::var(MAX_RETRIES_ENV_VAR) {
        Ok(val) => Ok(Some(val.parse()?)),
        Err(VarError::NotPresent) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Computes the maximum number of retries from the `max_retries` ini key.
/// Returns `Ok(None)` if the key is not set, other errors are returned as is.
fn max_retries_from_ini(ini: &Ini) -> Result<Option<u32>, ParseIntError> {
    ini.get_from(Some("http"), MAX_RETRIES_INI_KEY)
        .map(|val| val.parse())
        .transpose()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DiscardedAuthUrlValue {
    AuthToken,
    Url,
}

impl fmt::Display for DiscardedAuthUrlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscardedAuthUrlValue::AuthToken => write!(
                f,
                "Ignoring an auth token because the selected URL comes from a different \
                 configuration source. Configure the URL and auth token together in the same file \
                 or through CLI arguments and environment variables."
            ),
            DiscardedAuthUrlValue::Url => write!(
                f,
                "Ignoring a configured URL because the selected auth token comes from a different \
                 configuration source. Configure the URL and auth token together in the same file \
                 or through CLI arguments and environment variables."
            ),
        }
    }
}

/// URL and token selected after all file sources have been reconciled.
///
/// Runtime values are applied to this state without modifying the file-backed INI.
struct AuthAndUrl {
    url: Option<String>,
    token: Option<AuthToken>,
}

impl AuthAndUrl {
    fn from_ini(ini: &Ini) -> Self {
        Self {
            url: url_from_ini(ini),
            token: ini.get_from(Some("auth"), "token").map(AuthToken::from),
        }
    }

    /// Applies CLI-over-environment runtime values as one source without converting the token back
    /// into a string or storing runtime values in the file-backed INI.
    fn merge_runtime(
        &mut self,
        environment_url: Option<String>,
        environment_token: Option<AuthToken>,
        cli_url: Option<&str>,
        cli_token: Option<&AuthToken>,
    ) -> Option<DiscardedAuthUrlValue> {
        self.merge(AuthAndUrl {
            url: cli_url.map(str::to_owned).or(environment_url),
            token: cli_token.cloned().or(environment_token),
        })
    }

    /// Applies a higher-priority URL/token source using the shared source-separation rule.
    fn merge(&mut self, overlay: AuthAndUrl) -> Option<DiscardedAuthUrlValue> {
        let should_discard = reconcile_auth_url(self, &overlay);
        match should_discard {
            Some(DiscardedAuthUrlValue::AuthToken) => self.token = None,
            Some(DiscardedAuthUrlValue::Url) => self.url = None,
            None => {}
        }

        if let Some(url) = overlay.url {
            self.url = Some(url);
        }
        if let Some(token) = overlay.token {
            self.token = Some(token);
        }

        should_discard
    }
}

/// Returns whether a token is self-contained because it has a nonempty embedded URL.
fn token_has_embedded_url(token: &AuthToken) -> bool {
    token.payload().is_some_and(|data| !data.url.is_empty())
}

/// Determines which inherited value must be removed before applying another source.
///
/// Tokens with embedded URLs are exempt because the manual URL cannot redirect them; final config
/// construction always selects the URL embedded in the token.
fn reconcile_auth_url(base: &AuthAndUrl, overlay: &AuthAndUrl) -> Option<DiscardedAuthUrlValue> {
    match (&overlay.url, &overlay.token) {
        (Some(_), None)
            if base
                .token
                .as_ref()
                .is_some_and(|token| !token_has_embedded_url(token)) =>
        {
            Some(DiscardedAuthUrlValue::AuthToken)
        }
        (None, Some(token)) if !token_has_embedded_url(token) && base.url.is_some() => {
            Some(DiscardedAuthUrlValue::Url)
        }
        _ => None,
    }
}

/// Merges a higher-priority file source while keeping URL and token selection source-consistent.
///
/// URL/token reconciliation uses the same typed rule as runtime merging. All other values
/// retain the existing key-by-key INI merge behavior.
fn merge_config_source(base: &mut Ini, overlay: &Ini) -> Option<DiscardedAuthUrlValue> {
    let should_discard =
        reconcile_auth_url(&AuthAndUrl::from_ini(base), &AuthAndUrl::from_ini(overlay));
    match should_discard {
        Some(DiscardedAuthUrlValue::AuthToken) => {
            base.delete_from(Some("auth"), "token");
        }
        Some(DiscardedAuthUrlValue::Url) => {
            base.delete_from(Some("defaults"), "url");
        }
        None => {}
    }

    for (section, props) in overlay.iter() {
        for (key, value) in props.iter() {
            base.set_to(section, key.to_owned(), value.to_owned());
        }
    }

    should_discard
}

fn warn_about_conflicting_urls(token_url: &str, manually_configured_url: Option<&str>) {
    if let Some(manually_configured_url) = manually_configured_url {
        if manually_configured_url != token_url {
            warn!(
                "Using {token_url} (embedded in token) rather than manually-configured URL \
                {manually_configured_url}. To use {manually_configured_url}, please provide an \
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
            max_retries: self.max_retries,
        }
    }
}

fn url_from_ini(ini: &Ini) -> Option<String> {
    ini.get_from(Some("defaults"), "url")
        .map(|url| url.to_owned())
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
        remote.to_owned()
    } else {
        "origin".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;

    use super::*;

    const USER_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const EMBEDDED_URL_TOKEN: &str = "sntrys_\
        eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
        Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
        lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA";

    fn ini(url: Option<&str>, token: Option<&str>) -> Ini {
        let mut ini = Ini::new();
        if let Some(url) = url {
            ini.set_to(Some("defaults"), "url".into(), url.to_owned());
        }
        if let Some(token) = token {
            ini.set_to(Some("auth"), "token".into(), token.to_owned());
        }
        ini
    }

    #[test]
    fn reconcile_keeps_url_and_token_from_same_source() {
        let base = AuthAndUrl::from_ini(&ini(Some("https://global.invalid"), Some(USER_TOKEN)));
        let overlay =
            AuthAndUrl::from_ini(&ini(Some("https://project.invalid"), Some("project-token")));

        assert_eq!(reconcile_auth_url(&base, &overlay), None);
    }

    #[test]
    fn reconcile_url_discards_inherited_token() {
        let base = AuthAndUrl::from_ini(&ini(Some("https://global.invalid"), Some(USER_TOKEN)));
        let overlay = AuthAndUrl::from_ini(&ini(Some("https://project.invalid"), None));

        assert_eq!(
            reconcile_auth_url(&base, &overlay),
            Some(DiscardedAuthUrlValue::AuthToken)
        );
    }

    #[test]
    fn reconcile_token_discards_inherited_url() {
        let base = AuthAndUrl::from_ini(&ini(Some("https://global.invalid"), None));
        let overlay = AuthAndUrl::from_ini(&ini(None, Some(USER_TOKEN)));

        assert_eq!(
            reconcile_auth_url(&base, &overlay),
            Some(DiscardedAuthUrlValue::Url)
        );
    }

    #[test]
    fn reconcile_url_keeps_inherited_embedded_url_token() {
        let base = AuthAndUrl::from_ini(&ini(None, Some(EMBEDDED_URL_TOKEN)));
        let overlay = AuthAndUrl::from_ini(&ini(Some("https://project.invalid"), None));

        assert_eq!(reconcile_auth_url(&base, &overlay), None);
    }

    #[test]
    fn merge_config_source_without_url_or_token_keeps_inherited_pair() {
        let mut base = ini(Some("https://global.invalid"), Some(USER_TOKEN));
        let mut overlay = Ini::new();
        overlay.set_to(Some("http"), "verify_ssl".into(), "false".into());

        assert_eq!(merge_config_source(&mut base, &overlay), None);
        assert_eq!(
            base.get_from(Some("defaults"), "url"),
            Some("https://global.invalid")
        );
        assert_eq!(base.get_from(Some("auth"), "token"), Some(USER_TOKEN));
        assert_eq!(base.get_from(Some("http"), "verify_ssl"), Some("false"));
    }

    #[test]
    fn cli_and_environment_form_one_runtime_source() {
        let cli_token = AuthToken::from("cli-token");
        let mut auth_and_url = AuthAndUrl {
            url: Some("https://file.invalid".to_owned()),
            token: None,
        };

        assert_eq!(
            auth_and_url.merge_runtime(
                Some("https://environment.invalid".to_owned()),
                Some(AuthToken::from(USER_TOKEN)),
                Some("https://cli.invalid"),
                Some(&cli_token),
            ),
            None
        );
        assert_eq!(auth_and_url.url.as_deref(), Some("https://cli.invalid"));
        assert_eq!(
            auth_and_url
                .token
                .as_ref()
                .map(|token| token.raw().expose_secret().as_str()),
            Some("cli-token")
        );
    }

    #[test]
    fn persisted_auth_is_available_when_runtime_url_discards_it() {
        let ini = ini(Some("https://file.invalid"), Some(USER_TOKEN));
        let config = Config::from_file_and_auth(
            PathBuf::from(".sentryclirc"),
            ini,
            AuthAndUrl {
                url: Some("https://environment.invalid".to_owned()),
                token: None,
            },
        );

        assert!(config.get_auth().is_none());
        assert!(config.get_persisted_auth().is_some());
    }

    #[cfg(not(windows))]
    #[test]
    fn save_restricts_existing_file_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let filename = dir.path().join(".sentryclirc");
        fs::write(&filename, "[defaults]\nurl=https://sentry.io/\n").unwrap();
        fs::set_permissions(&filename, fs::Permissions::from_mode(0o644)).unwrap();

        Config::from_file(filename.clone(), Ini::new())
            .save()
            .unwrap();

        assert_eq!(
            fs::metadata(filename).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn test_get_api_endpoint() {
        let config = Config {
            filename: PathBuf::from("/path/to/config"),
            process_bound: false,
            ini: Default::default(),
            cached_auth: None,
            cached_base_url: "https://sentry.io/".to_owned(),
            cached_headers: None,
            cached_log_level: LevelFilter::Off,
            cached_vcs_remote: String::new(),
            cached_token_data: None,
            max_retries: 0,
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
