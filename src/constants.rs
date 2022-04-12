//! Provides some useful constants.

use std::time::Duration;

/// Application name
pub const APP_NAME: &str = "sentrycli";

/// The default API URL
pub const DEFAULT_URL: &str = "https://sentry.io/";

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
/// Default maximum file size of DIF uploads.
pub const DEFAULT_MAX_DIF_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB
/// Default maximum file size of a single file inside DIF bundle.
pub const DEFAULT_MAX_DIF_ITEM_SIZE: u64 = 1024 * 1024; // 1MB
/// Default maximum DIF upload size.
pub const DEFAULT_MAX_DIF_UPLOAD_SIZE: u64 = 35 * 1024 * 1024; // 35MB
/// Default maximum time to wait for file assembly.
pub const DEFAULT_MAX_WAIT: Duration = Duration::from_secs(5 * 60);

include!(concat!(env!("OUT_DIR"), "/constants.gen.rs"));
