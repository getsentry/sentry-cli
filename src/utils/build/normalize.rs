#[cfg(not(windows))]
use std::fs;
use std::fs::File;
use std::io::Write as _;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use crate::constants::VERSION;
use crate::utils::fs::TempFile;
use anyhow::{Context as _, Result};
use itertools::Itertools as _;
use log::debug;
use symbolic::common::ByteView;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{DateTime, ZipWriter};

fn get_version() -> String {
    #[cfg(test)]
    {
        std::env::var("SENTRY_CLI_VERSION_OVERRIDE").unwrap_or_else(|_| VERSION.to_owned())
    }
    #[cfg(not(test))]
    {
        VERSION.to_owned()
    }
}

fn sort_entries(path: &Path) -> Result<impl Iterator<Item = (PathBuf, PathBuf)>> {
    Ok(WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            // Include both regular files and symlinks
            path.is_file() || path.is_symlink()
        })
        .map(|entry| {
            let entry_path = entry.into_path();
            let relative_path = entry_path.strip_prefix(path)?.to_owned();
            Ok((entry_path, relative_path))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sorted_by(|(_, a), (_, b)| a.cmp(b)))
}

fn add_entries_to_zip(
    zip: &mut ZipWriter<File>,
    entries: impl Iterator<Item = (PathBuf, PathBuf)>,
    directory_name: &str,
) -> Result<i32> {
    let mut file_count = 0;

    // Need to set the last modified time to a fixed value to ensure consistent checksums
    // This is important as an optimization to avoid re-uploading the same chunks if they're already on the server
    // but the last modified time being different will cause checksums to be different.
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());

    for (entry_path, relative_path) in entries {
        #[cfg(not(windows))]
        // On Unix, we need to preserve the file permissions.
        let options = options.unix_permissions(fs::metadata(&entry_path)?.permissions().mode());

        let zip_path = format!("{directory_name}/{}", relative_path.to_string_lossy());

        if entry_path.is_symlink() {
            // Handle symlinks by reading the target path and writing it as a symlink
            let target = std::fs::read_link(&entry_path)?;
            let target_str = target.to_string_lossy();

            // Create a symlink entry in the zip
            zip.add_symlink(zip_path.as_str(), &target_str, options)
                .with_context(|| format!("Failed to add symlink '{zip_path}' to zip archive"))?;
        } else {
            // Handle regular files
            zip.start_file(zip_path.as_str(), options)
                .with_context(|| format!("Failed to add file '{zip_path}' to zip archive"))?;
            let file_byteview = ByteView::open(&entry_path)?;
            zip.write_all(file_byteview.as_slice())?;
        }
        file_count += 1;
    }

    Ok(file_count)
}

fn metadata_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(DateTime::default())
}

pub fn write_version_metadata<W: std::io::Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
) -> Result<()> {
    let version = get_version();
    zip.start_file(".sentry-cli-metadata.txt", metadata_file_options())?;
    writeln!(zip, "sentry-cli-version: {version}")?;
    Ok(())
}

// For XCArchive directories, we'll zip the entire directory
// It's important to not change the contents of the directory or the size
// analysis will be wrong and the code signature will break.
pub fn normalize_directory(path: &Path, parsed_assets_path: &Path) -> Result<TempFile> {
    debug!("Creating normalized zip for directory: {}", path.display());

    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let directory_name = path.file_name().expect("Failed to get basename");

    // Collect and sort entries for deterministic ordering
    // This is important to ensure stable sha1 checksums for the zip file as
    // an optimization is used to avoid re-uploading the same chunks if they're already on the server.
    let entries = sort_entries(path)?;
    let mut file_count = add_entries_to_zip(&mut zip, entries, &directory_name.to_string_lossy())?;

    // Add parsed assets to the zip in a "ParsedAssets" directory
    if parsed_assets_path.exists() {
        debug!(
            "Adding parsed assets from: {}",
            parsed_assets_path.display()
        );

        let parsed_assets_entries = sort_entries(parsed_assets_path)?;
        file_count += add_entries_to_zip(
            &mut zip,
            parsed_assets_entries,
            &format!("{}/ParsedAssets", directory_name.to_string_lossy()),
        )?;
    }

    write_version_metadata(&mut zip)?;

    zip.finish()?;
    debug!("Successfully created normalized zip for directory with {file_count} files");
    Ok(temp_file)
}
