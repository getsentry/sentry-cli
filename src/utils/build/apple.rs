use anyhow::{anyhow, bail, Context as _, Result};
use log::debug;
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::utils::fs::TempDir;
use apple_catalog_parsing;
use std::io::Cursor;
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn handle_asset_catalogs(archive_path: &Path, output_path: &Path) {
    // Find all asset catalogs
    let cars = find_car_files(archive_path);
    for car in &cars {
        if let Err(e) =
            apple_catalog_parsing::inspect_asset_catalog(car, &output_path.to_path_buf())
        {
            eprintln!("Failed to inspect asset catalog {}: {e}", car.display());
        }
    }
}

fn find_car_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq("car"))
        })
        .map(|e| e.into_path())
        .collect()
}

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
pub fn ipa_to_xcarchive(
    ipa_path: &Path,
    ipa_bytes: &[u8],
    temp_dir: &TempDir,
    dsym_paths: &[&Path],
) -> Result<PathBuf> {
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

    let app_name = extract_app_name_from_ipa(&ipa_archive)?.to_owned();

    // Extract all files from the archive
    for i in 0..ipa_archive.len() {
        let mut file = ipa_archive.by_index(i)?;

        if let Some(name) = file.enclosed_name() {
            if let Ok(stripped) = name.strip_prefix("Payload/") {
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

    copy_dsyms(dsym_paths, &xcarchive_dir)?;

    debug!(
        "Created XCArchive Info.plist at: {}",
        info_plist_path.display()
    );
    Ok(xcarchive_dir)
}

fn copy_dsyms(dsym_paths: &[&Path], xcarchive_dir: &Path) -> Result<()> {
    if dsym_paths.is_empty() {
        return Ok(());
    }

    let dsyms_dir = xcarchive_dir.join("dSYMs");
    std::fs::create_dir(&dsyms_dir)?;

    for dsym_input in dsym_paths {
        for dsym_path in resolve_dsym_bundles(dsym_input)? {
            let bundle_name = dsym_path
                .file_name()
                .ok_or_else(|| anyhow!("dSYM path has no bundle name: {}", dsym_path.display()))?;
            let destination = dsyms_dir.join(bundle_name);
            if destination.exists() {
                bail!(
                    "Cannot include multiple dSYM bundles named {}",
                    bundle_name.to_string_lossy()
                );
            }

            for entry in WalkDir::new(&dsym_path) {
                let entry = entry.with_context(|| {
                    format!("Failed to read dSYM bundle {}", dsym_path.display())
                })?;
                let relative_path = entry.path().strip_prefix(&dsym_path)?;
                let target_path = destination.join(relative_path);

                if entry.file_type().is_dir() {
                    std::fs::create_dir_all(&target_path)?;
                } else if entry.file_type().is_file() {
                    std::fs::copy(entry.path(), &target_path).with_context(|| {
                        format!(
                            "Failed to copy dSYM file {} to {}",
                            entry.path().display(),
                            target_path.display()
                        )
                    })?;
                } else if entry.file_type().is_symlink() {
                    let link_target = std::fs::read_link(entry.path())?;
                    std::os::unix::fs::symlink(link_target, &target_path)?;
                }
            }
        }
    }

    Ok(())
}

fn resolve_dsym_bundles(path: &Path) -> Result<Vec<PathBuf>> {
    if is_dsym_bundle(path) {
        return Ok(vec![path.to_owned()]);
    }

    if !path.is_dir() {
        bail!(
            "dSYM path must be a .dSYM bundle or a directory containing .dSYM bundles: {}",
            path.display()
        );
    }

    let mut bundles = Vec::new();
    for entry in std::fs::read_dir(path)
        .with_context(|| format!("Failed to read dSYM directory {}", path.display()))?
    {
        let entry =
            entry.with_context(|| format!("Failed to read dSYM directory {}", path.display()))?;
        let entry_path = entry.path();
        if is_dsym_bundle(&entry_path) {
            bundles.push(entry_path);
        }
    }

    if bundles.is_empty() {
        bail!("No .dSYM bundles found in directory: {}", path.display());
    }

    Ok(bundles)
}

fn is_dsym_bundle(path: &Path) -> bool {
    path.is_dir()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dsym"))
}

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Payload/([^/]+)\.app/Info\.plist$").expect("regex is valid"));

fn extract_app_name_from_ipa<'a>(archive: &'a ZipArchive<Cursor<&[u8]>>) -> Result<&'a str> {
    let matches = archive
        .file_names()
        .filter_map(|name| PATTERN.captures(name))
        .map(|c| c.get(1).expect("group 1 must be present").as_str())
        .take(2) // If there are ≥2 matches, we already know the IPA is invalid
        .collect::<Vec<_>>();

    if let &[app_name] = matches.as_slice() {
        Ok(app_name)
    } else {
        Err(anyhow!("IPA did not contain exactly one .app."))
    }
}
