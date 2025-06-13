#[cfg(target_os = "macos")]
mod asset_catalog;

#[cfg(target_os = "macos")]
pub use asset_catalog::inspect_asset_catalog;
