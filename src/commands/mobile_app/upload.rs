use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, bail, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::ProgressStyle;
use itertools::Itertools;
use log::{debug, info, warn};
use sha1_smol::Digest;
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::{DateTime, ZipWriter};

use crate::api::{Api, AuthenticatedApi, ChunkUploadCapability};
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::fs::get_sha1_checksums;
use crate::utils::fs::TempFile;
#[cfg(target_os = "macos")]
use crate::utils::mobile_app::handle_asset_catalogs;
use crate::utils::mobile_app::{is_aab_file, is_apk_file, is_apple_app, is_zip_file};
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
                .action(ArgAction::Append)
                .required(true),
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
        .get_one("sha")
        .map(String::as_str)
        .map(Cow::Borrowed)
        .or_else(|| vcs::find_head().ok().map(Cow::Owned));

    let build_configuration = matches.get_one("build_configuration").map(String::as_str);

    let api = Api::current();
    let authenticated_api = api.authenticated()?;

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

        #[cfg(target_os = "macos")]
        if is_apple_app(path) {
            handle_asset_catalogs(path);
        }

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

    let mut uploaded_paths = vec![];
    let mut errored_paths = vec![];
    for (path, zip) in normalized_zips {
        info!("Uploading file: {}", path.display());
        let bytes = ByteView::open(zip.path())?;
        match upload_file(
            &authenticated_api,
            &bytes,
            &org,
            &project,
            sha.as_deref(),
            build_configuration,
        ) {
            Ok(_) => {
                info!("Successfully uploaded file: {}", path.display());
                uploaded_paths.push(path.to_path_buf());
            }
            Err(e) => {
                debug!("Failed to upload file at path {}: {}", path.display(), e);
                errored_paths.push(path.to_path_buf());
            }
        }
    }

    if !errored_paths.is_empty() {
        warn!(
            "Failed to upload {} file{}:",
            errored_paths.len(),
            if errored_paths.len() == 1 { "" } else { "s" }
        );
        for path in errored_paths {
            warn!("  - {}", path.display());
        }
    }

    println!(
        "Successfully uploaded {} file{} to Sentry",
        uploaded_paths.len(),
        if uploaded_paths.len() == 1 { "" } else { "s" }
    );
    if uploaded_paths.len() < 3 {
        for path in &uploaded_paths {
            println!("  - {}", path.display());
        }
    }

    if uploaded_paths.is_empty() {
        bail!("Failed to upload any files");
    }
    Ok(())
}

fn validate_is_mobile_app(path: &Path, bytes: &[u8]) -> Result<()> {
    debug!("Validating mobile app format for: {}", path.display());

    if is_apple_app(path) {
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

    // Need to set the last modified time to a fixed value to ensure consistent checksums
    // This is important as an optimization to avoid re-uploading the same chunks if they're already on the server
    // but the last modified time being different will cause checksums to be different.
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(DateTime::default());

    zip.start_file(file_name, options)?;
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

    // Collect and sort entries for deterministic ordering
    // This is important to ensure stable sha1 checksums for the zip file as
    // an optimization is used to avoid re-uploading the same chunks if they're already on the server.
    let entries = walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| {
            let entry_path = entry.into_path();
            let relative_path = entry_path.strip_prefix(
                path.parent().ok_or_else(|| anyhow!("Cannot determine parent directory for path: {}", path.display()))?
            )?.to_owned();
            Ok((entry_path, relative_path))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sorted_by(|(_, a), (_, b)| a.cmp(b));

    // Need to set the last modified time to a fixed value to ensure consistent checksums
    // This is important as an optimization to avoid re-uploading the same chunks if they're already on the server
    // but the last modified time being different will cause checksums to be different.
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(DateTime::default());

    for (entry_path, relative_path) in entries {
        debug!("Adding file to zip: {}", relative_path.display());

        zip.start_file(relative_path.to_string_lossy(), options)?;
        let file_byteview = ByteView::open(&entry_path)?;
        zip.write_all(file_byteview.as_slice())?;
        file_count += 1;
    }

    zip.finish()?;
    debug!(
        "Successfully created normalized zip for directory with {} files",
        file_count
    );
    Ok(temp_file)
}

fn upload_file(
    api: &AuthenticatedApi,
    bytes: &[u8],
    org: &str,
    project: &str,
    sha: Option<&str>,
    build_configuration: Option<&str>,
) -> Result<()> {
    const SELF_HOSTED_ERROR_HINT: &str = "If you are using a self-hosted Sentry server, \
        update to the latest version of Sentry to use the mobile-app upload command.";

    debug!(
        "Uploading file to organization: {}, project: {}, sha: {}, build_configuration: {}",
        org,
        project,
        sha.unwrap_or("unknown"),
        build_configuration.unwrap_or("unknown")
    );

    let chunk_upload_options = api.get_chunk_upload_options(org)?.ok_or_else(|| {
        anyhow!(
            "The Sentry server lacks chunked uploading support, which \
                is required for mobile app uploads. {SELF_HOSTED_ERROR_HINT}"
        )
    })?;

    if !chunk_upload_options.supports(ChunkUploadCapability::PreprodArtifacts) {
        bail!(
            "The Sentry server lacks support for receiving files uploaded \
            with this command. {SELF_HOSTED_ERROR_HINT}"
        );
    }

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

    pb.finish_with_duration("Finishing upload");

    let response =
        api.assemble_mobile_app(org, project, checksum, &checksums, sha, build_configuration)?;
    chunks.retain(|Chunk((digest, _))| response.missing_chunks.contains(digest));

    if !chunks.is_empty() {
        let upload_progress_style = ProgressStyle::default_bar().template(
            "{prefix:.dim} Uploading files...\
             \n{wide_bar}  {bytes}/{total_bytes} ({eta})",
        );
        upload_chunks(&chunks, &chunk_upload_options, upload_progress_style)?;
    } else {
        println!("Nothing to upload, all files are on the server");
    }

    poll_assemble(
        api,
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
        info!("File upload complete (processing pending on server)");
    } else {
        info!("File processing complete");
    }

    Ok(())
}

#[cfg(not(windows))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use zip::ZipArchive;

    #[test]
    fn test_normalize_directory_preserves_top_level_directory_name() -> Result<()> {
        let temp_dir = crate::utils::fs::TempDir::create()?;
        let test_dir = temp_dir.path().join("MyApp.xcarchive");
        fs::create_dir_all(test_dir.join("Products"))?;
        fs::write(test_dir.join("Products").join("app.txt"), "test content")?;

        let result_zip = normalize_directory(&test_dir)?;
        let zip_file = fs::File::open(result_zip.path())?;
        let mut archive = ZipArchive::new(zip_file)?;
        let file = archive.by_index(0)?;
        let file_path = file.name();
        assert_eq!(file_path, "MyApp.xcarchive/Products/app.txt");
        Ok(())
    }
}
