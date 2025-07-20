#![cfg(feature = "unstable-mobile-app")]

use anyhow::{anyhow, Result};
use log::debug;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::utils::fs::TempDir;

/// Converts an IPA file to an XCArchive directory structure. The provided IPA must be a valid IPA file.
///
/// # Format Overview
///
/// ## IPA (iOS App Store Package)
/// An IPA file is a compressed archive containing an iOS app ready for distribution.
/// It has the following structure:
/// ```
/// MyApp.ipa
/// └── Payload/
///     └── MyApp.app/
///         ├── Info.plist
///         ├── MyApp (executable)
///         ├── Assets.car
///         └── ... (other app resources)
/// ```
///
/// ## XCArchive (Xcode Archive)
/// An XCArchive is a directory structure created by Xcode when archiving an app for distribution.
/// It has the following structure:
/// ```
/// MyApp.xcarchive/
/// ├── Info.plist
/// ├── Products/
/// │   └── Applications/
/// │       └── MyApp.app/
/// │           ├── Info.plist
/// │           ├── MyApp (executable)
/// │           ├── Assets.car
/// │           └── ... (other app resources)
/// └── ... (other archive metadata)
/// ```
pub fn ipa_to_xcarchive(ipa_path: &Path, ipa_bytes: &[u8], temp_dir: &TempDir) -> Result<PathBuf> {
    debug!(
        "Converting IPA to XCArchive structure: {}",
        ipa_path.display()
    );

    let xcarchive_dir = temp_dir.path().join("archive.xcarchive");
    let products_dir = xcarchive_dir.join("Products");
    let applications_dir = products_dir.join("Applications");

    debug!("Creating XCArchive directory structure");
    std::fs::create_dir_all(&applications_dir)?;

    // Extract IPA file
    let cursor = Cursor::new(ipa_bytes);
    let mut ipa_archive = ZipArchive::new(cursor)?;

    let app_name = extract_app_name_from_ipa(&ipa_archive)?;

    // Extract all files from the archive
    for i in 0..ipa_archive.len() {
        let mut file = ipa_archive.by_index(i)?;

        if let Some(stripped) = file
            .enclosed_name()
            .and_then(|name| name.strip_prefix("Payload/").ok())
        {
                if !file.is_dir() {
                    // Create the file path in the XCArchive structure
                    let target_path = applications_dir.join(stripped);

                    // Create parent directories if necessary
                    if let Some(parent) = target_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    // Extract file
                    let mut target_file = std::fs::File::create(&target_path)?;
                    std::io::copy(&mut file, &mut target_file)?;
                }
            }
        }
    }

    // Create Info.plist for XCArchive
    let info_plist_path = xcarchive_dir.join("Info.plist");

    let info_plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>ApplicationProperties</key>
	<dict>
		<key>ApplicationPath</key>
		<string>Applications/{app_name}.app</string>
	</dict>
	<key>ArchiveVersion</key>
	<integer>1</integer>
</dict>
</plist>"#
    );

    std::fs::write(&info_plist_path, info_plist_content)?;

    debug!(
        "Created XCArchive Info.plist at: {}",
        info_plist_path.display()
    );
    Ok(xcarchive_dir)
}

fn extract_app_name_from_ipa<'a>(archive: &'a ZipArchive<Cursor<&[u8]>>) -> Result<&'a str> {
    archive
        .file_names()
        .filter(|name| name.starts_with("Payload/") && name.ends_with(".app/Info.plist"))
        .min_by_key(|name| name.len())
        .and_then(|name| name.strip_prefix("Payload/"))
        .and_then(|name| name.split('/').next())
        .and_then(|name| name.strip_suffix(".app"))
        .ok_or_else(|| anyhow!("No .app found in IPA"))
}
