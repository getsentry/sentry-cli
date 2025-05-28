use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::Result;

pub fn is_zip_file<R: Read + Seek>(reader: &mut R) -> Result<bool> {
    let mut magic = [0u8; 4];
    reader.seek(SeekFrom::Start(0))?;

    if reader.read_exact(&mut magic).is_err() {
        return Ok(false);
    }

    // https://en.wikipedia.org/wiki/List_of_file_signatures
    const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
    const ZIP_MAGIC_EMPTY: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
    const ZIP_MAGIC_SPANNED: [u8; 4] = [0x50, 0x4B, 0x07, 0x08];
    Ok(magic == ZIP_MAGIC || magic == ZIP_MAGIC_EMPTY || magic == ZIP_MAGIC_SPANNED)
}

pub fn is_apk_file<R: Read + Seek>(reader: &mut R) -> Result<bool> {
    reader.seek(SeekFrom::Start(0))?;

    let mut archive = zip::ZipArchive::new(reader)?;

    // APK files must contain AndroidManifest.xml at the root of the zip file
    let has_manifest = archive.by_name("AndroidManifest.xml").is_ok();

    Ok(has_manifest)
}

pub fn is_aab_file<R: Read + Seek>(reader: &mut R) -> Result<bool> {
    reader.seek(SeekFrom::Start(0))?;

    let mut archive = zip::ZipArchive::new(reader)?;

    // AAB files must contain BundleConfig.pb and base/manifest/AndroidManifest.xml
    let has_bundle_config = archive.by_name("BundleConfig.pb").is_ok();
    let has_base_manifest = archive.by_name("base/manifest/AndroidManifest.xml").is_ok();

    Ok(has_bundle_config && has_base_manifest)
}

pub fn is_xcarchive_directory<P: AsRef<Path>>(path: P) -> Result<bool> {
    let path = path.as_ref();

    // XCArchive should have Info.plist and Products directory
    let info_plist = path.join("Info.plist");
    let products_dir = path.join("Products");

    Ok(info_plist.exists() && products_dir.exists() && products_dir.is_dir())
}
