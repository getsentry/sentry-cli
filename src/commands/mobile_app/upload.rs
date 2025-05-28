use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use anyhow::anyhow;
use anyhow::Result;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};

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

#[expect(clippy::unnecessary_wraps)]
pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path_strings = matches.get_many::<String>("paths").unwrap();

    let mut paths: Vec<&Path> = Vec::new();
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();

        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a file: {}", path.display()));
        }

        validate_mobile_app_file(path)?;
        paths.push(path);
    }

    for path in paths {
        println!("Uploading mobile app file: {}", path.display());
        // TODO: Upload the file to the chunked uploads API
    }

    eprintln!("Uploading mobile app files to a project is not yet implemented.");
    Ok(())
}

pub fn validate_mobile_app_file(path: &Path) -> Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // First check if the file is a zip file (AAB or APK)
    if is_zip_file(&mut reader)? {
        reader.seek(SeekFrom::Start(0))?;

        if is_aab_file(&mut reader)? {
            return Ok(());
        }

        reader.seek(SeekFrom::Start(0))?;
        if is_apk_file(&mut reader)? {
            return Ok(());
        }
    }

    // Check for XCArchive (directory)
    if path.is_dir() && is_xcarchive_directory(path)? {
        return Ok(());
    }

    Err(anyhow!(
        "File is not a recognized mobile app format (APK, AAB, or XCArchive): {}",
        path.display()
    ))
}
