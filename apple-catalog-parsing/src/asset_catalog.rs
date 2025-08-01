use std::ffi::CString;
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to convert path to C string: {0}")]
    PathConversion(#[from] std::ffi::NulError),
}

extern "C" {
    fn swift_inspect_asset_catalog(
        catalog_path: *const std::os::raw::c_char,
        output_path: *const std::os::raw::c_char,
    );
}

/// This calls out to Swift code that uses Apple APIs to convert the contents
/// of an asset catalog into a format that can be parsed by the
/// size analysis backend. It enables main size analysis features such
/// as duplicate image detection, xray, and image optimization insights.
/// The path should be in an xcarchive file, results are written
/// to a JSON file in the xcarchive’s ParsedAssets directory.
pub fn inspect_asset_catalog<P>(catalog_path: P, output_path: P) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let catalog_c_string = CString::new(catalog_path.as_ref().as_os_str().as_bytes())?;
    let output_path_c_string = CString::new(output_path.as_ref().as_os_str().as_bytes())?;
    let catalog_string_ptr = catalog_c_string.as_ptr();
    let output_string_ptr = output_path_c_string.as_ptr();
    unsafe {
        // The string pointed to is immutable, in Swift we cannot change it.
        // We ensure this by using "UnsafePointer<CChar>" in Swift which is
        // immutable (as opposed to "UnsafeMutablePointer<CChar>").
        swift_inspect_asset_catalog(catalog_string_ptr, output_string_ptr);
    }
    Ok(())
}
