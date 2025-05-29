use std::path::Path;

use anyhow::anyhow;
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

    let mut paths = Vec::new();
    let mut normalized_zips: Vec<TempFile> = Vec::new();
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        validate_is_mobile_app(path)?;
        let normalized_zip = normalize_mobile_app(path)?;
        normalized_zips.push(normalized_zip);
    }

    for zip in normalized_zips {
        println!("Created normalized zip at: {}", zip.path().display());
        // TODO: Upload the normalized zip to the chunked uploads API
    }

    eprintln!("Uploading mobile app files to a project is not yet implemented.");
    Ok(())
}

fn validate_is_mobile_app(path: &Path) -> Result<()> {
    // Check for XCArchive (directory) first
    if path.is_dir() && is_xcarchive_directory(path)? {
        return Ok(());
    }

    let byteview = ByteView::open(path)?;

    // Check if the file is a zip file (then AAB or APK)
    if is_zip_file(&byteview) {
        if is_aab_file(&byteview)? {
            return Ok(());
        }

        if is_apk_file(&byteview)? {
            return Ok(());
        }
    }

    Err(anyhow!(
        "File is not a recognized mobile app format (APK, AAB, or XCArchive): {}",
        path.display()
    ))
}

/// Normalizes a mobile app file into a zip file to ensure consistent artifact parsing logic on the backend.
fn normalize_mobile_app(path: &Path) -> Result<TempFile> {
    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    if path.is_file() {
        // For APK and AAB files, we'll copy them directly into the zip
        let mut file = File::open(path)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file name"))?
            .to_string_lossy();

        zip.start_file(file_name, SimpleFileOptions::default())?;
        std::io::copy(&mut file, &mut zip)?;
    } else if path.is_dir() {
        // For XCArchive directories, we'll zip the entire directory
        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let relative_path = entry_path
                    .strip_prefix(path)
                    .map_err(|_| anyhow!("Failed to get relative path"))?;

                let mut file = File::open(entry_path)?;
                zip.start_file(
                    relative_path.to_string_lossy(),
                    SimpleFileOptions::default(),
                )?;
                std::io::copy(&mut file, &mut zip)?;
            }
        }
    }

    zip.finish()?;
    Ok(temp_file)
}
