use std::borrow::Cow;
use std::io::Write as _;
use std::path::Path;

use anyhow::{anyhow, bail, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::ProgressStyle;
use log::{debug, info, warn};
use symbolic::common::ByteView;
use zip::write::SimpleFileOptions;
use zip::{DateTime, ZipWriter};

use crate::api::{
    Api, AuthenticatedApi, ChunkUploadCapability, ChunkedBuildRequest, ChunkedFileState, VcsInfo,
};
use crate::config::Config;
use crate::utils::args::ArgExt as _;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::utils::build::{handle_asset_catalogs, ipa_to_xcarchive, is_apple_app, is_ipa_file};
use crate::utils::build::{is_aab_file, is_apk_file, is_zip_file, normalize_directory};
use crate::utils::chunks::{upload_chunks, Chunk};
use crate::utils::fs::get_sha1_checksums;
use crate::utils::fs::TempDir;
use crate::utils::fs::TempFile;
use crate::utils::progress::ProgressBar;
use crate::utils::vcs::{
    self, get_github_base_ref, get_github_pr_number, get_provider_from_remote,
    get_repo_from_remote_preserve_case, git_repo_base_ref, git_repo_base_repo_name_preserve_case,
    git_repo_head_ref, git_repo_remote_url,
};

pub fn make_command(command: Command) -> Command {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    const HELP_TEXT: &str =
        "The path to the build to upload. Supported files include Apk, Aab, XCArchive, and IPA.";
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    const HELP_TEXT: &str =
        "The path to the build to upload. Supported files include Apk, and Aab.";
    command
        .about("[EXPERIMENTAL] Upload builds to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help(HELP_TEXT)
                .num_args(1..)
                .action(ArgAction::Append)
                .required(true),
        )
        .arg(
            Arg::new("head_sha")
                .long("head-sha")
                .help("The VCS commit sha to use for the upload. If not provided, the current commit sha will be used.")
        )
        .arg(
            Arg::new("base_sha")
                .long("base-sha")
                .help("The VCS commit's base sha to use for the upload. If not provided, the merge-base of the current and remote branch will be used.")
        )
        .arg(
            Arg::new("vcs_provider")
                .long("vcs-provider")
                .help("The VCS provider to use for the upload. If not provided, the current provider will be used.")
        )
        .arg(
            Arg::new("head_repo_name")
                .long("head-repo-name")
                .help("The name of the git repository to use for the upload (e.g. organization/repository). If not provided, the current repository will be used.")
        )
        .arg(
            Arg::new("base_repo_name")
                .long("base-repo-name")
                .help("The name of the git repository to use for the upload (e.g. organization/repository). If not provided, the current repository will be used.")
        )
        .arg(
            Arg::new("head_ref")
                .long("head-ref")
                .help("The reference (branch) to use for the upload. If not provided, the current reference will be used.")
        )
        .arg(
            Arg::new("base_ref")
                .long("base-ref")
                .help("The base reference (branch) to use for the upload. If not provided, the merge-base with the remote tracking branch will be used.")
        )
        .arg(
            Arg::new("pr_number")
                .long("pr-number")
                .value_parser(clap::value_parser!(u32))
                .help("The pull request number to use for the upload. If not provided and running \
                    in a pull_request-triggered GitHub Actions workflow, the PR number will be automatically \
                    detected from GitHub Actions environment variables.")
        )
        .arg(
            Arg::new("build_configuration")
                .long("build-configuration")
                .help("The build configuration to use for the upload. If not provided, the current version will be used.")
        )
        .arg(
            Arg::new("release_notes")
                .long("release-notes")
                .help("The release notes to use for the upload.")
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let path_strings = matches
        .get_many::<String>("paths")
        .expect("paths argument is required");

    let head_sha = matches
        .get_one("head_sha")
        .map(String::as_str)
        .map(Cow::Borrowed)
        .or_else(|| vcs::find_head().ok().map(Cow::Owned));

    let cached_remote = config.get_cached_vcs_remote();
    // Try to open the git repository and find the remote, but handle errors gracefully.
    let (vcs_provider, head_repo_name, head_ref, base_ref, base_repo_name) = {
        // Try to open the repo and get the remote URL, but don't fail if not in a repo.
        let repo = git2::Repository::open_from_env().ok();
        let repo_ref = repo.as_ref();
        let remote_url = repo_ref.and_then(|repo| git_repo_remote_url(repo, &cached_remote).ok());

        let vcs_provider = matches
            .get_one("vcs_provider")
            .map(String::as_str)
            .map(Cow::Borrowed)
            .or_else(|| {
                remote_url
                    .as_ref()
                    .map(|url| get_provider_from_remote(url))
                    .map(Cow::Owned)
            });

        let head_repo_name = matches
            .get_one("head_repo_name")
            .map(String::as_str)
            .map(Cow::Borrowed)
            .or_else(|| {
                remote_url
                    .as_ref()
                    .map(|url| get_repo_from_remote_preserve_case(url))
                    .map(Cow::Owned)
            });

        let head_ref = matches
            .get_one("head_ref")
            .map(String::as_str)
            .map(Cow::Borrowed)
            .or_else(|| {
                // Try to get the current ref from the VCS if not provided
                // Note: git_repo_head_ref will return an error for detached HEAD states,
                // which the error handling converts to None - this prevents sending "HEAD" as a branch name
                // In that case, the user will need to provide a valid branch name.
                repo_ref
                    .and_then(|r| match git_repo_head_ref(r) {
                        Ok(ref_name) => {
                            debug!("Found current branch reference: {}", ref_name);
                            Some(ref_name)
                        }
                        Err(e) => {
                            debug!(
                                "No valid branch reference found (likely detached HEAD): {}",
                                e
                            );
                            None
                        }
                    })
                    .map(Cow::Owned)
            });

        let base_ref = matches
            .get_one("base_ref")
            .map(String::as_str)
            .map(Cow::Borrowed)
            .or_else(|| {
                // First try GitHub Actions environment variables
                get_github_base_ref().map(Cow::Owned)
            })
            .or_else(|| {
                // Fallback to git repository introspection
                repo_ref
                    .and_then(|r| match git_repo_base_ref(r, &cached_remote) {
                        Ok(base_ref_name) => {
                            debug!("Found base reference: {}", base_ref_name);
                            Some(base_ref_name)
                        }
                        Err(e) => {
                            warn!("Could not detect base branch reference: {}", e);
                            None
                        }
                    })
                    .map(Cow::Owned)
            });

        let base_repo_name = matches
            .get_one("base_repo_name")
            .map(String::as_str)
            .map(Cow::Borrowed)
            .or_else(|| {
                // Try to get the base repo name from the VCS if not provided
                repo_ref
                    .and_then(|r| match git_repo_base_repo_name_preserve_case(r) {
                        Ok(Some(base_repo_name)) => {
                            debug!("Found base repository name: {}", base_repo_name);
                            Some(base_repo_name)
                        }
                        Ok(None) => {
                            debug!("No base repository found - not a fork");
                            None
                        }
                        Err(e) => {
                            warn!("Could not detect base repository name: {}", e);
                            None
                        }
                    })
                    .map(Cow::Owned)
            });

        (
            vcs_provider,
            head_repo_name,
            head_ref,
            base_ref,
            base_repo_name,
        )
    };
    let base_sha = matches.get_one("base_sha").map(String::as_str);
    let pr_number = matches
        .get_one("pr_number")
        .copied()
        .or_else(get_github_pr_number);

    let build_configuration = matches.get_one("build_configuration").map(String::as_str);
    let release_notes = matches.get_one("release_notes").map(String::as_str);

    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    debug!("Starting upload for {} paths", path_strings.len());

    let mut normalized_zips = vec![];
    for path_string in path_strings {
        let path: &Path = path_string.as_ref();
        debug!("Processing artifact at path: {}", path.display());

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        let byteview = ByteView::open(path)?;
        debug!("Loaded file with {} bytes", byteview.len());

        validate_is_supported_build(path, &byteview)?;

        let normalized_zip = if path.is_file() {
            debug!("Normalizing file: {}", path.display());
            handle_file(path, &byteview)?
        } else if path.is_dir() {
            debug!("Normalizing directory: {}", path.display());
            handle_directory(path).with_context(|| {
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

    let mut uploaded_paths_and_urls = vec![];
    let mut errored_paths_and_reasons = vec![];
    for (path, zip) in normalized_zips {
        info!("Uploading file: {}", path.display());
        let bytes = ByteView::open(zip.path())?;
        let vcs_info = VcsInfo {
            head_sha: head_sha.as_deref(),
            base_sha,
            vcs_provider: vcs_provider.as_deref(),
            head_repo_name: head_repo_name.as_deref(),
            base_repo_name: base_repo_name.as_deref(),
            head_ref: head_ref.as_deref(),
            base_ref: base_ref.as_deref(),
            pr_number: pr_number.as_ref(),
        };
        match upload_file(
            &authenticated_api,
            &bytes,
            &org,
            &project,
            build_configuration,
            release_notes,
            &vcs_info,
        ) {
            Ok(artifact_url) => {
                info!("Successfully uploaded file: {}", path.display());
                uploaded_paths_and_urls.push((path.to_path_buf(), artifact_url));
            }
            Err(e) => {
                debug!("Failed to upload file at path {}: {}", path.display(), e);
                errored_paths_and_reasons.push((path.to_path_buf(), e));
            }
        }
    }

    if !errored_paths_and_reasons.is_empty() {
        warn!(
            "Failed to upload {} file{}:",
            errored_paths_and_reasons.len(),
            if errored_paths_and_reasons.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        for (path, reason) in errored_paths_and_reasons {
            warn!("  - {}", path.display());
            warn!("    Error: {reason:#}");
        }
    }

    if uploaded_paths_and_urls.is_empty() {
        bail!("Failed to upload any files");
    } else {
        println!(
            "Successfully uploaded {} file{} to Sentry",
            uploaded_paths_and_urls.len(),
            if uploaded_paths_and_urls.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        if uploaded_paths_and_urls.len() < 3 {
            for (path, artifact_url) in &uploaded_paths_and_urls {
                println!("  - {} ({artifact_url})", path.display());
            }
        }
    }
    Ok(())
}

fn handle_file(path: &Path, byteview: &ByteView) -> Result<TempFile> {
    // Handle IPA files by converting them to XCArchive
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if is_zip_file(byteview) && is_ipa_file(byteview)? {
        debug!("Converting IPA file to XCArchive structure");
        let archive_temp_dir = TempDir::create()?;
        return ipa_to_xcarchive(path, byteview, &archive_temp_dir)
            .and_then(|path| handle_directory(&path))
            .with_context(|| format!("Failed to process IPA file {}", path.display()));
    }

    normalize_file(path, byteview).with_context(|| {
        format!(
            "Failed to generate uploadable bundle for file {}",
            path.display()
        )
    })
}

fn validate_is_supported_build(path: &Path, bytes: &[u8]) -> Result<()> {
    debug!("Validating build format for: {}", path.display());

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if is_apple_app(path) {
        debug!("Detected XCArchive directory");
        return Ok(());
    }

    // Check if the file is a zip file (then AAB, APK, or IPA)
    if is_zip_file(bytes) {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        debug!("File is a zip, checking for AAB/APK/IPA format");
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        debug!("File is a zip, checking for AAB/APK format");

        if is_aab_file(bytes)? {
            debug!("Detected AAB file");
            return Ok(());
        }

        if is_apk_file(bytes)? {
            debug!("Detected APK file");
            return Ok(());
        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if is_ipa_file(bytes)? {
            debug!("Detected IPA file");
            return Ok(());
        }
    }

    debug!("File format validation failed");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let format_list = "APK, AAB, XCArchive, or IPA";
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    let format_list = "APK, or AAB";

    Err(anyhow!(
        "File is not a recognized supported build format ({format_list}): {}",
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

fn handle_directory(path: &Path) -> Result<TempFile> {
    let temp_dir = TempDir::create()?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if is_apple_app(path) {
        handle_asset_catalogs(path, temp_dir.path());
    }
    normalize_directory(path, temp_dir.path())
}

/// Returns artifact url if upload was successful.
fn upload_file(
    api: &AuthenticatedApi,
    bytes: &[u8],
    org: &str,
    project: &str,
    build_configuration: Option<&str>,
    release_notes: Option<&str>,
    vcs_info: &VcsInfo<'_>,
) -> Result<String> {
    const SELF_HOSTED_ERROR_HINT: &str = "If you are using a self-hosted Sentry server, \
        update to the latest version of Sentry to use the build upload command.";

    debug!(
        "Uploading file to organization: {}, project: {}, build_configuration: {}, vcs_info: {:?}",
        org,
        project,
        build_configuration.unwrap_or("unknown"),
        vcs_info,
    );

    let chunk_upload_options = api.get_chunk_upload_options(org)?.ok_or_else(|| {
        anyhow!(
            "The Sentry server lacks chunked uploading support, which \
                is required for build uploads. {SELF_HOSTED_ERROR_HINT}"
        )
    })?;

    if !chunk_upload_options.supports(ChunkUploadCapability::PreprodArtifacts) {
        bail!(
            "The Sentry server lacks support for receiving files uploaded \
            with this command. {SELF_HOSTED_ERROR_HINT}"
        );
    }

    let progress_style =
        ProgressStyle::default_spinner().template("{spinner} Preparing for upload...");
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

    pb.finish_with_duration("Preparing for upload");

    // In the normal case we go through this loop exactly twice:
    // 1. state=not_found
    //    server tells us the we need to send every chunk and we do so
    // 2. artifact_url set so we're done (likely state=created)
    //
    // In the case where all the chunks are already on the server we go
    // through only once:
    // 1. state=created, artifact_url set
    //
    // In the case where something went wrong (which could be on either
    // iteration of the loop) we get:
    // n. state=error, artifact_url unset

    let result = loop {
        let response = api.assemble_build(
            org,
            project,
            &ChunkedBuildRequest {
                checksum,
                chunks: &checksums,
                build_configuration,
                release_notes,
                head_sha: vcs_info.head_sha,
                base_sha: vcs_info.base_sha,
                provider: vcs_info.vcs_provider,
                head_repo_name: vcs_info.head_repo_name,
                base_repo_name: vcs_info.base_repo_name,
                head_ref: vcs_info.head_ref,
                base_ref: vcs_info.base_ref,
                pr_number: vcs_info.pr_number,
            },
        )?;
        chunks.retain(|Chunk((digest, _))| response.missing_chunks.contains(digest));

        if !chunks.is_empty() {
            let upload_progress_style = ProgressStyle::default_bar().template(
                "{prefix:.dim} Uploading files...\
               \n{wide_bar}  {bytes}/{total_bytes} ({eta})",
            );
            upload_chunks(&chunks, &chunk_upload_options, upload_progress_style)?;
        }

        // state.is_err() is not the same as this since it also returns
        // true for ChunkedFileState::NotFound.
        if response.state == ChunkedFileState::Error {
            let message = response.detail.as_deref().unwrap_or("unknown error");
            bail!("Failed to process uploaded files: {}", message);
        }

        if let Some(artifact_url) = response.artifact_url {
            break Ok(artifact_url);
        }

        if response.state.is_finished() {
            bail!("File upload is_finished() but did not succeeded or error");
        }
    };

    result
}

#[cfg(not(windows))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use zip::ZipArchive;

    #[test]
    fn test_normalize_directory_preserves_top_level_directory_name() -> Result<()> {
        let temp_dir = crate::utils::fs::TempDir::create()?;
        let test_dir = temp_dir.path().join("MyApp.xcarchive");
        fs::create_dir_all(test_dir.join("Products"))?;
        fs::write(test_dir.join("Products").join("app.txt"), "test content")?;

        let result_zip = normalize_directory(&test_dir, temp_dir.path())?;
        let zip_file = fs::File::open(result_zip.path())?;
        let mut archive = ZipArchive::new(zip_file)?;
        let file = archive.by_index(0)?;
        let file_path = file.name();
        assert_eq!(file_path, "MyApp.xcarchive/Products/app.txt");
        Ok(())
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_xcarchive_upload_includes_parsed_assets() -> Result<()> {
        // Test that XCArchive uploads include parsed asset catalogs
        let xcarchive_path = Path::new("tests/integration/_fixtures/build/archive.xcarchive");

        // Process the XCArchive directory
        let result = handle_directory(xcarchive_path)?;

        // Verify the resulting zip contains parsed assets
        let zip_file = fs::File::open(result.path())?;
        let mut archive = ZipArchive::new(zip_file)?;

        let mut has_parsed_assets = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file
                .enclosed_name()
                .ok_or(anyhow!("Failed to get file name"))?;
            if file_name.to_string_lossy().contains("ParsedAssets") {
                has_parsed_assets = true;
                break;
            }
        }

        assert!(
            has_parsed_assets,
            "XCArchive upload should include parsed asset catalogs"
        );
        Ok(())
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_ipa_upload_includes_parsed_assets() -> Result<()> {
        // Test that IPA uploads handle missing asset catalogs gracefully
        let ipa_path = Path::new("tests/integration/_fixtures/build/ipa_with_asset.ipa");
        let byteview = ByteView::open(ipa_path)?;

        // Process the IPA file - this should work even without asset catalogs
        let result = handle_file(ipa_path, &byteview)?;

        let zip_file = fs::File::open(result.path())?;
        let mut archive = ZipArchive::new(zip_file)?;

        let mut has_parsed_assets = false;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file
                .enclosed_name()
                .ok_or(anyhow!("Failed to get file name"))?;
            if file_name.to_string_lossy().contains("ParsedAssets") {
                has_parsed_assets = true;
                break;
            }
        }

        assert!(
            has_parsed_assets,
            "XCArchive upload should include parsed asset catalogs"
        );
        Ok(())
    }

    #[test]
    fn test_normalize_directory_preserves_symlinks() -> Result<()> {
        let temp_dir = crate::utils::fs::TempDir::create()?;
        let test_dir = temp_dir.path().join("TestApp.xcarchive");
        fs::create_dir_all(test_dir.join("Products"))?;

        // Create a regular file
        fs::write(test_dir.join("Products").join("app.txt"), "test content")?;

        // Create a symlink pointing to the regular file
        let symlink_path = test_dir.join("Products").join("app_link.txt");
        symlink("app.txt", &symlink_path)?;

        let result_zip = normalize_directory(&test_dir, temp_dir.path())?;
        let zip_file = fs::File::open(result_zip.path())?;
        let mut archive = ZipArchive::new(zip_file)?;

        // Check that both the regular file and symlink are in the zip
        let mut has_regular_file = false;
        let mut has_symlink = false;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file.name();

            if file_name == "TestApp.xcarchive/Products/app.txt" {
                has_regular_file = true;
                // Verify it's actually a regular file, not a symlink
                assert!(
                    !file.is_symlink(),
                    "app.txt should be a regular file, not a symlink"
                );
            } else if file_name == "TestApp.xcarchive/Products/app_link.txt" {
                has_symlink = true;
                // Verify it's actually a symlink
                assert!(
                    file.is_symlink(),
                    "app_link.txt should be a symlink in the zip"
                );
            }
        }

        assert!(has_regular_file, "Regular file should be in zip");
        assert!(has_symlink, "Symlink should be preserved in zip");
        Ok(())
    }
}
