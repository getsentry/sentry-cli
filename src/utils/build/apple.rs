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
/// Any provided dSYM inputs are included in the generated XCArchive.
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
    dsym_paths: &[&Path],
    temp_dir: &TempDir,
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
        copy_dsym_input(dsym_input, &dsyms_dir)?;
    }

    Ok(())
}

fn copy_dsym_input(dsym_input: &Path, dsyms_dir: &Path) -> Result<()> {
    let metadata = match dsym_input.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            bail!("dSYM path does not exist: {}", dsym_input.display());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to access dSYM path {}", dsym_input.display()));
        }
    };

    if metadata.file_type().is_symlink() {
        bail!("dSYM paths cannot be symlinks: {}", dsym_input.display());
    }

    let extracted = if metadata.is_file() {
        Some(extract_dsym_zip(dsym_input)?)
    } else if metadata.is_dir() {
        None
    } else {
        bail!(
            "dSYM path must be a .dSYM bundle, a directory containing dSYM bundles, or a ZIP archive: {}",
            dsym_input.display()
        );
    };

    let root = extracted
        .as_ref()
        .map_or(dsym_input, |temp_dir| temp_dir.path());
    let bundles = discover_dsym_bundles(root, extracted.is_some())?;
    if bundles.is_empty() {
        let input_kind = if extracted.is_some() {
            "ZIP archive"
        } else {
            "directory"
        };
        bail!(
            "No .dSYM bundles found in {input_kind}: {}",
            dsym_input.display()
        );
    }

    for dsym_path in bundles {
        copy_dsym_bundle(&dsym_path, dsyms_dir)?;
    }

    Ok(())
}

fn copy_dsym_bundle(dsym_path: &Path, dsyms_dir: &Path) -> Result<()> {
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

    debug!(
        "Including dSYM bundle in IPA upload: {}",
        dsym_path.display()
    );

    for entry in WalkDir::new(dsym_path) {
        let entry =
            entry.with_context(|| format!("Failed to read dSYM bundle {}", dsym_path.display()))?;
        let relative_path = entry.path().strip_prefix(dsym_path)?;
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
            bail!(
                "Symlinks are not supported in dSYM bundles: {}",
                entry.path().display()
            );
        }
    }

    Ok(())
}

fn extract_dsym_zip(path: &Path) -> Result<TempDir> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open dSYM ZIP {}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("dSYM input is not a valid ZIP archive: {}", path.display()))?;
    let temp_dir = TempDir::create()?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let entry_path = entry
            .enclosed_name()
            .ok_or_else(|| anyhow!("dSYM ZIP contains an unsafe path: {}", entry.name()))?;

        if entry.is_symlink() {
            bail!(
                "Symlinks are not supported in dSYM ZIP archives: {}",
                entry.name()
            );
        }

        // Ignore common archive metadata so it does not affect dSYM layout discovery.
        if !zip::read::root_dir_common_filter(&entry_path) {
            continue;
        }

        let target_path = temp_dir.path().join(entry_path);
        if entry.is_dir() {
            std::fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut target_file = std::fs::File::create(&target_path)?;
            std::io::copy(&mut entry, &mut target_file)?;
        }
    }

    Ok(temp_dir)
}

fn discover_dsym_bundles(path: &Path, allow_wrapper: bool) -> Result<Vec<PathBuf>> {
    if has_dsym_extension(path) {
        return Ok(vec![path.to_owned()]);
    }

    let mut bundles = Vec::new();
    let mut directories = Vec::new();
    for entry in std::fs::read_dir(path)
        .with_context(|| format!("Failed to read dSYM directory {}", path.display()))?
    {
        let entry =
            entry.with_context(|| format!("Failed to read dSYM directory {}", path.display()))?;
        let entry_path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() && has_dsym_extension(&entry_path) {
            bail!("dSYM paths cannot be symlinks: {}", entry_path.display());
        }
        if file_type.is_dir() {
            if has_dsym_extension(&entry_path) {
                bundles.push(entry_path);
            } else {
                directories.push(entry_path);
            }
        }
    }

    if bundles.is_empty() && allow_wrapper && directories.len() == 1 {
        return discover_dsym_bundles(&directories[0], false);
    }

    Ok(bundles)
}

fn has_dsym_extension(path: &Path) -> bool {
    path.extension()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::os::unix::fs::symlink;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_dsym(root: &Path, name: &str, contents: &str) -> Result<PathBuf> {
        let bundle = root.join(name);
        std::fs::create_dir_all(&bundle)?;
        std::fs::write(bundle.join("symbols"), contents)?;
        Ok(bundle)
    }

    fn create_dsym_zip(path: &Path, wrapper: Option<&str>, bundles: &[(&str, &str)]) -> Result<()> {
        let mut archive = ZipWriter::new(std::fs::File::create(path)?);
        for (name, contents) in bundles {
            let entry = match wrapper {
                Some(wrapper) => format!("{wrapper}/{name}/symbols"),
                None => format!("{name}/symbols"),
            };
            archive.start_file(entry, SimpleFileOptions::default())?;
            archive.write_all(contents.as_bytes())?;
        }
        archive.finish()?;
        Ok(())
    }

    fn create_output_dir(root: &Path, name: &str) -> Result<PathBuf> {
        let output = root.join(name);
        std::fs::create_dir(&output)?;
        Ok(output)
    }

    #[test]
    fn copy_dsyms_accepts_bundle_and_directory_inputs() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let direct = create_dsym(temp_dir.path(), "DemoApp.app.dSYM", "app symbols")?;
        let symbols_dir = temp_dir.path().join("Symbols");
        create_dsym(
            &symbols_dir,
            "DemoFramework.framework.dSYM",
            "framework symbols",
        )?;
        std::fs::write(symbols_dir.join("README.txt"), "ignored")?;
        let xcarchive = create_output_dir(temp_dir.path(), "archive.xcarchive")?;

        copy_dsyms(&[direct.as_path(), symbols_dir.as_path()], &xcarchive)?;

        let output = xcarchive.join("dSYMs");
        assert_eq!(
            std::fs::read_to_string(output.join("DemoApp.app.dSYM/symbols"))?,
            "app symbols"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("DemoFramework.framework.dSYM/symbols"))?,
            "framework symbols"
        );
        assert!(!output.join("README.txt").exists());
        Ok(())
    }

    #[test]
    fn copy_dsyms_accepts_supported_zip_layouts() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let bundle_zip = temp_dir.path().join("bundle.zip");
        create_dsym_zip(&bundle_zip, None, &[("DemoApp.app.dSYM", "app symbols")])?;
        let directory_zip = temp_dir.path().join("directory.zip");
        create_dsym_zip(
            &directory_zip,
            Some("dSYMs"),
            &[("DemoFramework.framework.dSYM", "framework symbols")],
        )?;
        let xcarchive = create_output_dir(temp_dir.path(), "archive.xcarchive")?;

        copy_dsyms(&[bundle_zip.as_path(), directory_zip.as_path()], &xcarchive)?;

        let output = xcarchive.join("dSYMs");
        assert_eq!(
            std::fs::read_to_string(output.join("DemoApp.app.dSYM/symbols"))?,
            "app symbols"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("DemoFramework.framework.dSYM/symbols"))?,
            "framework symbols"
        );
        Ok(())
    }

    #[test]
    fn copy_dsyms_ignores_macos_metadata_in_zip() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let zip = temp_dir.path().join("symbols.zip");
        let mut archive = ZipWriter::new(std::fs::File::create(&zip)?);
        archive.start_file(
            "dSYMs/DemoApp.app.dSYM/symbols",
            SimpleFileOptions::default(),
        )?;
        archive.write_all(b"symbols")?;
        archive.start_file(
            "__MACOSX/dSYMs/DemoApp.app.dSYM/._symbols",
            SimpleFileOptions::default(),
        )?;
        archive.write_all(b"metadata")?;
        archive.finish()?;
        let xcarchive = create_output_dir(temp_dir.path(), "archive.xcarchive")?;

        copy_dsyms(&[zip.as_path()], &xcarchive)?;

        let output = xcarchive.join("dSYMs/DemoApp.app.dSYM");
        assert_eq!(std::fs::read_to_string(output.join("symbols"))?, "symbols");
        assert!(!output.join("._symbols").exists());
        Ok(())
    }

    #[test]
    fn copy_dsym_input_rejects_missing_input() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let output = create_output_dir(temp_dir.path(), "output")?;
        let error = copy_dsym_input(&temp_dir.path().join("missing.dSYM"), &output).unwrap_err();
        assert!(format!("{error:#}").contains("dSYM path does not exist"));
        Ok(())
    }

    #[test]
    fn copy_dsym_input_rejects_inputs_without_dsyms() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let empty_directory = create_output_dir(temp_dir.path(), "empty")?;
        let output = create_output_dir(temp_dir.path(), "directory-output")?;
        let error = copy_dsym_input(&empty_directory, &output).unwrap_err();
        assert!(format!("{error:#}").contains("No .dSYM bundles found in directory"));

        let empty_zip = temp_dir.path().join("empty.zip");
        ZipWriter::new(std::fs::File::create(&empty_zip)?).finish()?;
        let output = create_output_dir(temp_dir.path(), "zip-output")?;
        let error = copy_dsym_input(&empty_zip, &output).unwrap_err();
        assert!(format!("{error:#}").contains("No .dSYM bundles found in ZIP archive"));
        Ok(())
    }

    #[test]
    fn copy_dsym_input_rejects_invalid_zip() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let zip = temp_dir.path().join("invalid.zip");
        std::fs::write(&zip, "not a ZIP")?;
        let output = create_output_dir(temp_dir.path(), "output")?;
        let error = copy_dsym_input(&zip, &output).unwrap_err();
        assert!(format!("{error:#}").contains("dSYM input is not a valid ZIP archive"));
        Ok(())
    }

    #[test]
    fn copy_dsym_input_rejects_unsafe_zip_entries() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let traversal_zip = temp_dir.path().join("traversal.zip");
        let mut archive = ZipWriter::new(std::fs::File::create(&traversal_zip)?);
        archive.start_file("../DemoApp.app.dSYM/symbols", SimpleFileOptions::default())?;
        archive.write_all(b"symbols")?;
        archive.finish()?;
        let output = create_output_dir(temp_dir.path(), "traversal-output")?;
        let error = copy_dsym_input(&traversal_zip, &output).unwrap_err();
        assert!(format!("{error:#}").contains("dSYM ZIP contains an unsafe path"));

        let symlink_zip = temp_dir.path().join("symlink.zip");
        let mut archive = ZipWriter::new(std::fs::File::create(&symlink_zip)?);
        archive.add_symlink(
            "DemoApp.app.dSYM/symbols",
            "../symbols",
            SimpleFileOptions::default(),
        )?;
        archive.finish()?;
        let output = create_output_dir(temp_dir.path(), "symlink-output")?;
        let error = copy_dsym_input(&symlink_zip, &output).unwrap_err();
        assert!(format!("{error:#}").contains("Symlinks are not supported in dSYM ZIP archives"));
        Ok(())
    }

    #[test]
    fn copy_dsym_input_rejects_symlink() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let bundle = create_dsym(temp_dir.path(), "DemoApp.app.dSYM", "symbols")?;
        let link = temp_dir.path().join("DemoAppAlias.app.dSYM");
        symlink(bundle, &link)?;
        let output = create_output_dir(temp_dir.path(), "output")?;
        let error = copy_dsym_input(&link, &output).unwrap_err();
        assert!(format!("{error:#}").contains("dSYM paths cannot be symlinks"));
        Ok(())
    }

    #[test]
    fn copy_dsym_bundle_rejects_internal_symlink() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let bundle = create_dsym(temp_dir.path(), "DemoApp.app.dSYM", "symbols")?;
        symlink("symbols", bundle.join("symbols-link"))?;
        let output = create_output_dir(temp_dir.path(), "output")?;
        let error = copy_dsym_bundle(&bundle, &output).unwrap_err();
        assert!(format!("{error:#}").contains("Symlinks are not supported in dSYM bundles"));
        Ok(())
    }

    #[test]
    fn copy_dsyms_rejects_duplicate_bundle_names() -> Result<()> {
        let temp_dir = TempDir::create()?;
        let first = create_dsym(&temp_dir.path().join("first"), "DemoApp.app.dSYM", "first")?;
        let second = create_dsym(
            &temp_dir.path().join("second"),
            "DemoApp.app.dSYM",
            "second",
        )?;
        let xcarchive = create_output_dir(temp_dir.path(), "archive.xcarchive")?;
        let error = copy_dsyms(&[first.as_path(), second.as_path()], &xcarchive).unwrap_err();
        assert!(format!("{error:#}")
            .contains("Cannot include multiple dSYM bundles named DemoApp.app.dSYM"));
        Ok(())
    }
}
