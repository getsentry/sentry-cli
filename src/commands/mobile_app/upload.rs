use std::path::Path;

use anyhow::anyhow;
use anyhow::Result;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use symbolic::common::ByteView;

use crate::utils::args::ArgExt;
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
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        validate_is_mobile_app(path)?;
        paths.push(path);
    }

    for path in paths {
        println!("Uploading mobile app file: {}", path.display());
        // TODO: Normalize the path to be a zip of the underlying file/dir
        // TODO: Upload the file to the chunked uploads API
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
