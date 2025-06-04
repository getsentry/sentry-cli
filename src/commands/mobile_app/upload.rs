use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::utils::args::ArgExt;
use crate::utils::fs::TempFile;
use crate::utils::mobile_app::{is_aab_file, is_apk_file, is_xcarchive_directory, is_zip_file};

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Upload mobile app files to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("The path to the mobile app files to upload. Supported files include Apk, Aab or XCArchive.")
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path_strings = matches
        .get_many::<String>("paths")
        .expect("paths argument is required");

    let mut normalized_zips = Vec::new();
    for path_string in path_strings {
        println!("Processing path: {}", path_string);
        let path: &Path = path_string.as_ref();

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        let byteview = ByteView::open(path)?;

        validate_is_mobile_app(path, &byteview)?;

        let normalized_zip = if path.is_file() {
            normalize_file(path, &byteview)
        } else if path.is_dir() {
            normalize_directory(path)
        } else {
            Err(anyhow!(
                "Path {} is neither a file nor a directory, cannot upload",
                path.display()
            ))
        }?;

        normalized_zips.push(normalized_zip);
    }

    for zip in normalized_zips {
        println!("Created normalized zip at: {}", zip.path().display());
        // TODO: Upload the normalized zip to the chunked uploads API
    }

    eprintln!("Uploading mobile app files to a project is not yet implemented.");
    Ok(())
}

fn validate_is_mobile_app(path: &Path, bytes: &[u8]) -> Result<()> {
    // Check for XCArchive (directory) first
    if path.is_dir() && is_xcarchive_directory(path) {
        return Ok(());
    }

    // Check if the file is a zip file (then AAB or APK)
    if is_zip_file(bytes) {
        if is_aab_file(bytes)? {
            return Ok(());
        }

        if is_apk_file(bytes)? {
            return Ok(());
        }
    }

    Err(anyhow!(
        "File is not a recognized mobile app format (APK, AAB, or XCArchive): {}",
        path.display()
    ))
}

// For APK and AAB files, we'll copy them directly into the zip
fn normalize_file(path: &Path, bytes: &[u8]) -> Result<TempFile> {
    println!("Normalizing file: {}", path.display());
    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let file_name = path
        .file_name()
        .unwrap()
        .to_str()
        .with_context(|| format!("Failed to get relative path for {}", path.display()))?;

    zip.start_file(file_name, SimpleFileOptions::default())?;
    zip.write(bytes)?;

    zip.finish()?;
    Ok(temp_file)
}

// For XCArchive directories, we'll zip the entire directory
fn normalize_directory(path: &Path) -> Result<TempFile> {
    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            let relative_path = entry_path.strip_prefix(path).with_context(|| {
                format!("Failed to get relative path for {}", entry_path.display())
            })?;

            zip.start_file(
                relative_path.to_string_lossy(),
                SimpleFileOptions::default(),
            )?;
            let file_byteview = ByteView::open(entry_path)?;
            zip.write_all(file_byteview.as_slice())?;
        }
    }

    zip.finish()?;
    Ok(temp_file)
}
