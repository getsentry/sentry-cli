use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::debug;
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::fs::get_sha1_checksums;
use crate::utils::fs::TempFile;
use crate::utils::mobile_app::{is_aab_file, is_apk_file, is_xcarchive_directory, is_zip_file};
use crate::utils::progress::ProgressBar;

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

    debug!(
        "Starting mobile app upload for {} paths",
        path_strings.len()
    );

    let mut normalized_zips = vec![];
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();
        debug!("Processing artifact at path: {}", path.display());

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        let byteview = ByteView::open(path)?;
        debug!("Loaded file with {} bytes", byteview.len());

        validate_is_mobile_app(path, &byteview)?;

        let normalized_zip = if path.is_file() {
            debug!("Normalizing file: {}", path.display());
            normalize_file(path, &byteview).with_context(|| {
                format!(
                    "Failed to generate uploadable bundle for file {}",
                    path.display()
                )
            })?
        } else if path.is_dir() {
            debug!("Normalizing directory: {}", path.display());
            normalize_directory(path).with_context(|| {
                format!(
                    "Failed to generate uploadable bundle for directory {}",
                    path.display()
                )
            })?
        } else {
            Err(anyhow!(
                "Path {} is neither a file nor a directory, cannot upload",
                path.display()
            ))?
        };

        debug!(
            "Successfully normalized to: {}",
            normalized_zip.path().display()
        );
        normalized_zips.push(normalized_zip);
    }

    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let chunk_upload_options = authenticated_api
        .get_chunk_upload_options(&org)?
        .expect("Chunked uploading is not supported for this organization");

    for zip in normalized_zips {
        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} Uploading files...\
       \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
            style(">").dim(),
        ));

        let byteview = ByteView::open(zip.path())?;
        let (checksum, checksums) =
            get_sha1_checksums(&byteview, chunk_upload_options.chunk_size as usize)?;
        let mut chunks = byteview
            .chunks(chunk_upload_options.chunk_size as usize)
            .zip(checksums.iter())
            .map(|(data, checksum)| Chunk((*checksum, data)))
            .collect::<Vec<_>>();

        let response = authenticated_api
            .assemble_mobile_app(&org, &project, checksum, &checksums, None, None)?;
        chunks.retain(|Chunk((digest, _))| response.missing_chunks.contains(digest));

        if !chunks.is_empty() {
            upload_chunks(&chunks, &chunk_upload_options, progress_style)?;
            println!("{} Uploaded files to Sentry", style(">").dim());
        } else {
            println!(
                "{} Nothing to upload, all files are on the server",
                style(">").dim()
            );
        }
        poll_assemble(checksum, &checksums, &org, &project)?;
    }

    // eprintln!("Uploading mobile app files to a project is not yet implemented.");
    Ok(())
}

fn validate_is_mobile_app(path: &Path, bytes: &[u8]) -> Result<()> {
    debug!("Validating mobile app format for: {}", path.display());

    // Check for XCArchive (directory) first
    if path.is_dir() && is_xcarchive_directory(path) {
        debug!("Detected XCArchive directory");
        return Ok(());
    }

    // Check if the file is a zip file (then AAB or APK)
    if is_zip_file(bytes) {
        debug!("File is a zip, checking for AAB/APK format");
        if is_aab_file(bytes)? {
            debug!("Detected AAB file");
            return Ok(());
        }

        if is_apk_file(bytes)? {
            debug!("Detected APK file");
            return Ok(());
        }
    }

    debug!("File format validation failed");
    Err(anyhow!(
        "File is not a recognized mobile app format (APK, AAB, or XCArchive): {}",
        path.display()
    ))
}

// For APK and AAB files, we'll copy them directly into the zip
fn normalize_file(path: &Path, bytes: &[u8]) -> Result<TempFile> {
    debug!("Creating normalized zip for file: {}", path.display());

    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let file_name = path
        .file_name()
        .expect("Failed to get file name")
        .to_str()
        .with_context(|| format!("Failed to get relative path for {}", path.display()))?;

    debug!("Adding file to zip: {}", file_name);
    zip.start_file(file_name, SimpleFileOptions::default())?;
    zip.write_all(bytes)?;

    zip.finish()?;
    debug!("Successfully created normalized zip for file");
    Ok(temp_file)
}

// For XCArchive directories, we'll zip the entire directory
fn normalize_directory(path: &Path) -> Result<TempFile> {
    debug!("Creating normalized zip for directory: {}", path.display());

    let temp_file = TempFile::create()?;
    let mut zip = ZipWriter::new(temp_file.open()?);

    let mut file_count = 0;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let entry_path = entry.path();
        if entry_path.is_file() {
            let relative_path = entry_path.strip_prefix(path)?;
            debug!("Adding file to zip: {}", relative_path.display());

            zip.start_file(
                relative_path.to_string_lossy(),
                SimpleFileOptions::default(),
            )?;
            let file_byteview = ByteView::open(entry_path)?;
            zip.write_all(file_byteview.as_slice())?;
            file_count += 1;
        }
    }

    zip.finish()?;
    debug!(
        "Successfully created normalized zip for directory with {} files",
        file_count
    );
    Ok(temp_file)
}

fn poll_assemble(checksum: Digest, checksums: &[Digest], org: &str, project: &str) -> Result<()> {
    let progress_style = ProgressStyle::default_spinner().template("{spinner} Processing files...");

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);
    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    let response = loop {
        let response: crate::api::AssembleMobileAppResponse = authenticated_api
            .assemble_mobile_app(&org, &project, checksum, &checksums, None, None)?;

        if response.state.is_finished() {
            break response;
        }

        std::thread::sleep(ASSEMBLE_POLL_INTERVAL);
    };

    if response.state.is_err() {
        let message = response.detail.as_deref().unwrap_or("unknown error");
        bail!("Failed to process uploaded files: {}", message);
    }

    pb.finish_with_duration("Processing");

    if response.state.is_pending() {
        println!(
            "{} File upload complete (processing pending on server)",
            style(">").dim()
        );
    } else {
        println!("{} File processing complete", style(">").dim());
    }

    // print_upload_context_details(context);

    Ok(())
}
