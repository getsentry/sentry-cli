//! Provides some useful constants.

use app_dirs::AppInfo;

pub const APP_INFO: &'static AppInfo = &AppInfo {
    name: "sentrycli",
    author: "Sentry",
};

/// The default API URL
pub const DEFAULT_URL: &'static str = "https://sentry.io/";

/// The protocol version of the library.
pub const PROTOCOL_VERSION: u32 = 6;

/// The version of the library
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// The name of the configuration file.
pub const CONFIG_RC_FILE_NAME: &'static str = ".sentryclirc";

/// The file extension of the binary (.exe or empty string)
#[cfg(windows)]
pub const EXT: &'static str = ".exe";

/// The file extension of the binary (.exe or empty string)
#[cfg(not(windows))]
pub const EXT: &'static str = "";

/// The DSN to emit sentry events to.
/*
#[cfg(feature = "with_crash_reporting")]
lazy_static! {
    pub static ref INTERNAL_SENTRY_DSN: ::sentry::Dsn =
        "https://4b5ba00d320841efbb18a330cf539f4a@sentry.io/1192882".parse().unwrap();
}
*/

include!(concat!(env!("OUT_DIR"), "/constants.gen.rs"));
