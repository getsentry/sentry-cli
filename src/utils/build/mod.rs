#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple;
mod normalize;
mod validation;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::apple::{handle_asset_catalogs, ipa_to_xcarchive};
pub use self::normalize::normalize_directory;
pub use self::validation::{is_aab_file, is_apk_file, is_zip_file};
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use self::validation::{is_apple_app, is_ipa_file};
