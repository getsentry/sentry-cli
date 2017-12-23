//! Provides some useful constants.

use app_dirs::AppInfo;

pub const APP_INFO: &'static AppInfo = &AppInfo {
    name: "sentrycli",
    author: "Sentry",
};

/// The default API URL
pub const DEFAULT_URL: &'static str = "https://sentry.io/";
/// The default device family
#[cfg(windows)]
pub const DEFAULT_FAMILY: &'static str = "Windows device";
#[cfg(not(windows))]
pub const DEFAULT_FAMILY: &'static str = "Unix device";


/// The protocol version of the library.
pub const PROTOCOL_VERSION: u32 = 6;

/// The version of the library
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// The file extension of the binary (.exe or empty string)
#[cfg(windows)]
pub const EXT: &'static str = ".exe";

/// The file extension of the binary (.exe or empty string)
#[cfg(not(windows))]
pub const EXT: &'static str = "";

include!(concat!(env!("OUT_DIR"), "/constants.gen.rs"));
