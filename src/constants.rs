//! Provides some useful constants.

use app_dirs::AppInfo;

pub const APP_INFO: &AppInfo = &AppInfo {
    name: "sentrycli",
    author: "Sentry",
};

/// The default API URL
pub const DEFAULT_URL: &str = "https://sentry.io/";

/// The protocol version of the library.
pub const PROTOCOL_VERSION: u32 = 6;

/// The version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the configuration file.
pub const CONFIG_RC_FILE_NAME: &str = ".sentryclirc";

/// The release registry URL where the latest released version of sentry-cli can be found
pub const RELEASE_REGISTRY_LATEST_URL: &str =
    "https://release-registry.services.sentry.io/apps/sentry-cli/latest";

/// The file extension of the binary (.exe or empty string)
#[cfg(windows)]
pub const EXT: &str = ".exe";

/// The file extension of the binary (.exe or empty string)
#[cfg(not(windows))]
pub const EXT: &str = "";

/// The DSN to emit sentry events to.
/*
#[cfg(feature = "with_crash_reporting")]
lazy_static! {
    pub static ref INTERNAL_SENTRY_DSN: sentry::Dsn =
        "https://4b5ba00d320841efbb18a330cf539f4a@sentry.io/1192882".parse().unwrap();
}
*/

/// Backoff multiplier (1.5 which is 50% increase per backoff).
pub const DEFAULT_MULTIPLIER: f64 = 1.5;
/// Backoff randomization factor (0 means no randomization).
pub const DEFAULT_RANDOMIZATION: f64 = 0.1;
/// Initial backoff interval in milliseconds.
pub const DEFAULT_INITIAL_INTERVAL: u64 = 1000;
/// Maximum backoff interval in milliseconds.
pub const DEFAULT_MAX_INTERVAL: u64 = 5000;
/// Default number of retry attempts
pub const DEFAULT_RETRIES: u32 = 5;

include!(concat!(env!("OUT_DIR"), "/constants.gen.rs"));
