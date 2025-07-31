#![cfg(feature = "unstable-mobile-app")]

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple;
mod validation;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::apple::{handle_asset_catalogs, ipa_to_xcarchive};
pub use self::validation::{is_aab_file, is_apk_file, is_zip_file};
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::validation::{is_apple_app, is_ipa_file};
