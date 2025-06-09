use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, bail, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;
use indicatif::ProgressStyle;
use log::debug;
use sha1_smol::Digest;
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::api::{Api, AuthenticatedApi};
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::fs::get_sha1_checksums;
use crate::utils::fs::TempFile;
use crate::utils::mobile_app::{is_aab_file, is_apk_file, is_xcarchive_directory, is_zip_file};
use crate::utils::progress::ProgressBar;
use crate::utils::vcs;

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
        .arg(
            Arg::new("sha")
                .long("sha")
                .help("The git commit sha to use for the upload. If not provided, the current commit sha will be used.")
        )
        .arg(
            Arg::new("build_configuration")
                .long("build-configuration")
                .help("The build configuration to use for the upload. If not provided, the current version will be used.")
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path_strings = matches
        .get_many::<String>("paths")
        .expect("paths argument is required");

    let sha = matches
        .get_one::<String>("sha")
        .cloned()
        .or_else(|| vcs::find_head().ok());

    let build_configuration = matches
        .get_one::<String>("build_configuration")
        .map(String::as_str);

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
        normalized_zips.push((path, normalized_zip));
    }

    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;

    for (path, zip) in normalized_zips {
        println!("Uploading file: {}", zip.path().display());
        let bytes = ByteView::open(zip.path())?;
        upload_file(&bytes, &org, &project, sha.as_deref(), build_configuration)
            .with_context(|| format!("Failed to upload file at path {}", path.display()))?;
        println!("Successfully uploaded file at path {}", path.display());
    }

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

fn upload_file(
    bytes: &[u8],
    org: &str,
    project: &str,
    sha: Option<&str>,
    build_configuration: Option<&str>,
) -> Result<()> {
    debug!(
        "Uploading file to organization: {}, project: {}, sha: {}, build_configuration: {}",
        org,
        project,
        sha.unwrap_or("unknown"),
        build_configuration.unwrap_or("unknown")
    );

    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    let chunk_upload_options = authenticated_api
        .get_chunk_upload_options(org)?
        .expect("Chunked uploading is not supported for this organization");

    let progress_style =
        ProgressStyle::default_spinner().template("{spinner} Optimizing bundle for upload...");
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let chunk_size = chunk_upload_options.chunk_size as usize;
    let (checksum, checksums) = get_sha1_checksums(bytes, chunk_size)?;
    let mut chunks = bytes
        .chunks(chunk_size)
        .zip(checksums.iter())
        .map(|(data, checksum)| Chunk((*checksum, data)))
        .collect::<Vec<_>>();

    // TODO
    pb.finish_with_duration("Optimizing");

    let response = authenticated_api.assemble_mobile_app(
        org,
        project,
        checksum,
        &checksums,
        sha,
        build_configuration,
    )?;
    chunks.retain(|Chunk((digest, _))| response.missing_chunks.contains(digest));

    if !chunks.is_empty() {
        let upload_progress_style = ProgressStyle::default_bar().template(
            "{prefix:.dim} Uploading files...\
             \n{wide_bar}  {bytes}/{total_bytes} ({eta})",
        );
        upload_chunks(&chunks, &chunk_upload_options, upload_progress_style)?;
        println!("{} Uploaded files to Sentry", style(">").dim());
    } else {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );
    }

    poll_assemble(
        &authenticated_api,
        checksum,
        &checksums,
        org,
        project,
        sha,
        build_configuration,
    )?;
    Ok(())
}

fn poll_assemble(
    api: &AuthenticatedApi,
    checksum: Digest,
    chunks: &[Digest],
    org: &str,
    project: &str,
    sha: Option<&str>,
    build_configuration: Option<&str>,
) -> Result<()> {
    debug!("Polling assemble for checksum: {}", checksum);

    let progress_style = ProgressStyle::default_spinner().template("{spinner} Processing files...");
    let pb = ProgressBar::new_spinner();

    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let response = loop {
        let response =
            api.assemble_mobile_app(org, project, checksum, chunks, sha, build_configuration)?;

        if response.state.is_finished() {
            break response;
        }

        std::thread::sleep(ASSEMBLE_POLL_INTERVAL);
    };

    pb.finish_with_duration("Processing");

    if response.state.is_err() {
        let message = response.detail.as_deref().unwrap_or("unknown error");
        bail!("Failed to process uploaded files: {}", message);
    }

    if response.state.is_pending() {
        println!(
            "{} File upload complete (processing pending on server)",
            style(">").dim()
        );
    } else {
        println!("{} File processing complete", style(">").dim());
    }

    Ok(())
}
