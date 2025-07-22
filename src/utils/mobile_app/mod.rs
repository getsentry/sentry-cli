#![cfg(feature = "unstable-mobile-app")]

#[cfg(target_os = "macos")]
mod apple;
mod validation;

#[cfg(target_os = "macos")]
pub use self::apple::{handle_asset_catalogs, ipa_to_xcarchive};
#[cfg(target_os = "macos")]
pub use self::validation::is_ipa_file;
pub use self::validation::{is_aab_file, is_apk_file, is_apple_app, is_zip_file};
