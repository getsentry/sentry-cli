use std::path::Path;

use anyhow::Result;

pub fn is_zip_file(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }

    let magic = &bytes[0..4];

    // https://en.wikipedia.org/wiki/List_of_file_signatures
    const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
    const ZIP_MAGIC_EMPTY: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
    const ZIP_MAGIC_SPANNED: [u8; 4] = [0x50, 0x4B, 0x07, 0x08];
    magic == ZIP_MAGIC || magic == ZIP_MAGIC_EMPTY || magic == ZIP_MAGIC_SPANNED
}

pub fn is_apk_file(bytes: &[u8]) -> Result<bool> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    // APK files must contain AndroidManifest.xml at the root of the zip file
    let has_manifest = archive.by_name("AndroidManifest.xml").is_ok();

    Ok(has_manifest)
}

pub fn is_aab_file(bytes: &[u8]) -> Result<bool> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    // AAB files must contain BundleConfig.pb and base/manifest/AndroidManifest.xml
    let has_bundle_config = archive.by_name("BundleConfig.pb").is_ok();
    let has_base_manifest = archive.by_name("base/manifest/AndroidManifest.xml").is_ok();

    Ok(has_bundle_config && has_base_manifest)
}

pub fn is_xcarchive_directory<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    // XCArchive should have Info.plist and a Products/ directory
    let info_plist = path.join("Info.plist");
    let products_dir = path.join("Products");

    info_plist.exists() && products_dir.exists() && products_dir.is_dir()
}

/// A path is an Apple app if it points to an xarchive directory
pub fn is_apple_app(path: &Path) -> bool {
    path.is_dir() && is_xcarchive_directory(path)
}
