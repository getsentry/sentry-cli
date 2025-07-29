#![cfg(feature = "unstable-mobile-app")]

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple;
mod validation;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::apple::{handle_asset_catalogs, ipa_to_xcarchive};
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::validation::is_ipa_file;
pub use self::validation::{is_aab_file, is_apk_file, is_apple_app, is_zip_file};
