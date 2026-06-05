use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result};
use backon::BackoffBuilder as _;
use clap::{Arg, ArgMatches, Command};
use console::style;
use futures_util::StreamExt as _;
use itertools::Itertools as _;
use log::{debug, warn};
use objectstore_client::{ClientBuilder, Error, ExpirationPolicy, OperationResult, Usecase};
use rayon::prelude::*;
use secrecy::ExposeSecret as _;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::{Api, CreateSnapshotResponse, ImageMetadata, SnapshotsManifest};
use crate::config::Config;
use crate::utils::args::ArgExt as _;
use crate::utils::build_vcs::collect_git_metadata;
use crate::utils::ci::is_ci;
use crate::utils::fs::IMAGE_EXTENSIONS;
use crate::utils::retry::{get_default_backoff, DurationAsMilliseconds as _};

const EXPERIMENTAL_WARNING: &str =
    "[EXPERIMENTAL] The \"snapshots upload\" command is experimental. \
    The command is subject to breaking changes, including removal, in any Sentry CLI release.";
const MAX_PIXELS_PER_IMAGE: u64 = 40_000_000;

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Upload snapshots to a project.")
        .long_about(format!(
            "Upload snapshots to a project.\n\n{EXPERIMENTAL_WARNING}"
        ))
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .help("The path to the folder containing images to upload.")
                .required(true),
        )
        .arg(
            Arg::new("app_id")
                .long("app-id")
                .value_name("APP_ID")
                .help("The application identifier.")
                .required(true),
        )
        .arg(
            Arg::new("diff_threshold")
                .long("diff-threshold")
                .value_name("THRESHOLD")
                .value_parser(|s: &str| {
                    let v: f64 = s.parse().map_err(|e| format!("invalid float: {e}"))?;
                    if !(0.0..=1.0).contains(&v) {
                        return Err("value must be between 0.0 and 1.0".to_owned());
                    }
                    Ok(v)
                })
                .help(
                    "If set, Sentry will only report images as changed if their \
                     difference % is greater than this value. \
                     Example: 0.01 = only report image changes >= 1%.",
                ),
        )
        .arg(
            Arg::new("selective")
                .long("selective")
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Indicates this upload contains only a subset of images. \
                     Removals and renames cannot be detected on PRs.",
                ),
        )
        .arg(
            Arg::new("all_image_file_names")
                .long("all-image-file-names")
                .value_name("NAMES")
                .conflicts_with("all_image_file_names_file")
                .help(
                    "Comma-separated list of all image names (including subdirectory paths) \
                     in the full test suite. \
                     Used with selective uploads to detect image removals and renames. \
                     Implicitly enables --selective.",
                ),
        )
        .arg(
            Arg::new("all_image_file_names_file")
                .long("all-image-file-names-file")
                .value_name("PATH")
                .conflicts_with("all_image_file_names")
                .help(
                    "Path to a file containing all image names (including subdirectory paths), \
                     one per line. \
                     Used with selective uploads to detect image removals and renames. \
                     Implicitly enables --selective.",
                ),
        )
        .git_metadata_args()
}

struct ImageInfo {
    path: PathBuf,
    relative_path: PathBuf,
    width: u32,
    height: u32,
}

impl ImageInfo {
    fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    eprintln!("{EXPERIMENTAL_WARNING}");

    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;

    let path = matches
        .get_one::<String>("path")
        .expect("path argument is required");
    let app_id = matches
        .get_one::<String>("app_id")
        .expect("app_id argument is required");
    let dir_path = Path::new(path);
    if !dir_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir_path.display());
    }

    // Collect git metadata if running in CI, unless explicitly enabled or disabled.
    let should_collect_git_metadata =
        matches.get_flag("force_git_metadata") || (!matches.get_flag("no_git_metadata") && is_ci());

    // Always collect git metadata, but only perform automatic inference when enabled
    let vcs_info = collect_git_metadata(matches, &config, should_collect_git_metadata);

    if vcs_info.pr_number.is_some() && vcs_info.base_sha.is_none() {
        anyhow::bail!(
            "A PR number was provided but no base SHA could be determined. \
             Snapshot comparisons require a base SHA to identify the base build. \
             Pass --base-sha explicitly or ensure your CI environment exposes the merge base."
        );
    }

    debug!("Scanning for images in: {}", dir_path.display());
    debug!("Organization: {org}");
    debug!("Project: {project}");

    // Collect image files and read their dimensions
    let images = collect_images(dir_path);
    if images.is_empty() {
        println!("{} No image files found", style("!").yellow());
        return Ok(());
    }

    println!(
        "{} Found {} image {}",
        style(">").dim(),
        style(images.len()).yellow(),
        if images.len() == 1 { "file" } else { "files" }
    );

    validate_image_sizes(&images)?;

    let all_image_file_names = parse_all_image_file_names(matches)?;

    let selective = matches.get_flag("selective") || all_image_file_names.is_some();

    if let Some(ref all_names) = all_image_file_names {
        let all_names_set: HashSet<&str> = all_names.iter().map(|s| s.as_str()).collect();
        let mut unknown: Vec<String> = images
            .iter()
            .map(|img| crate::utils::fs::path_as_url(&img.relative_path))
            .filter(|k| !all_names_set.contains(k.as_str()))
            .collect();
        if !unknown.is_empty() {
            unknown.sort();
            anyhow::bail!(
                "The following uploaded images are not in --all-image-file-names: {}",
                unknown.join(", ")
            );
        }
    }

    println!(
        "{} Processing {} image {}",
        style(">").dim(),
        style(images.len()).yellow(),
        if images.len() == 1 { "file" } else { "files" }
    );

    let manifest_entries = upload_images(images, &org, &project)?;

    // Build manifest from discovered images
    let diff_threshold = matches.get_one::<f64>("diff_threshold").copied();

    let manifest = SnapshotsManifest {
        app_id: app_id.clone(),
        images: manifest_entries,
        diff_threshold,
        selective,
        all_image_file_names,
        vcs_info,
    };

    // POST manifest to API
    println!("{} Creating snapshot...", style(">").dim());
    let api = Api::current();
    let response: CreateSnapshotResponse = api
        .authenticated()?
        .create_preprod_snapshot(&org, &project, &manifest)?
        .convert()?;

    println!(
        "{} Created snapshot {} with {} {}",
        style(">").dim(),
        style(&response.artifact_id).yellow(),
        style(response.image_count).yellow(),
        if response.image_count == 1 {
            "image"
        } else {
            "images"
        }
    );

    if let Some(url) = &response.snapshot_url {
        println!(
            "{} View snapshots at {}",
            style(">").dim(),
            style(url).cyan()
        );
    }

    Ok(())
}

fn split_and_trim(input: &str, separator: char) -> Vec<String> {
    input
        .split(separator)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn normalize_image_names(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .map(|s| s.strip_prefix("./").unwrap_or(&s).replace('\\', "/"))
        .collect()
}

fn parse_all_image_file_names(matches: &ArgMatches) -> Result<Option<Vec<String>>> {
    if let Some(names_str) = matches.get_one::<String>("all_image_file_names") {
        let names = normalize_image_names(split_and_trim(names_str, ','));
        if names.is_empty() {
            anyhow::bail!("--all-image-file-names must not be empty");
        }
        return Ok(Some(names));
    }

    if let Some(file_path) = matches.get_one::<String>("all_image_file_names_file") {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read --all-image-file-names-file: {file_path}"))?;
        let names = normalize_image_names(split_and_trim(&content, '\n'));
        if names.is_empty() {
            anyhow::bail!(
                "--all-image-file-names-file is empty or contains only blank lines: {file_path}"
            );
        }
        return Ok(Some(names));
    }

    Ok(None)
}

fn collect_images(dir: &Path) -> Vec<ImageInfo> {
    WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(dir, e.path()))
        .filter_map(|res| match res {
            Ok(entry) => Some(entry),
            Err(err) => {
                warn!("Failed to access file during directory walk: {err}");
                None
            }
        })
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| is_image_file(entry.path()))
        .filter_map(|entry| collect_image_info(dir, entry.path()))
        .collect()
}

/// Builds [`ImageInfo`] for a discovered image path during snapshot collection.
///
/// Returns `Some(ImageInfo)` when the image dimensions can be parsed, or `None`
/// when the file should be skipped (e.g. when dimensions cannot be determined).
fn collect_image_info(dir: &Path, path: &Path) -> Option<ImageInfo> {
    let (width, height) = match imagesize::size(path) {
        Ok(dims) => (dims.width as u32, dims.height as u32),
        Err(err) => {
            warn!("Could not read dimensions from {}: {err}", path.display());
            return None;
        }
    };
    let relative = path.strip_prefix(dir).unwrap_or(path).to_path_buf();

    Some(ImageInfo {
        path: path.to_path_buf(),
        relative_path: relative,
        width,
        height,
    })
}

fn validate_image_sizes(images: &[ImageInfo]) -> Result<()> {
    let mut violations = images
        .iter()
        .filter(|img| img.pixels() > MAX_PIXELS_PER_IMAGE)
        .map(|img| {
            let path = img.relative_path.display();
            let width = img.width;
            let height = img.height;
            let pixels = img.pixels();

            format!("  {path} ({width}x{height} = {pixels} pixels)")
        })
        .peekable();

    if violations.peek().is_some() {
        let violation_messages = violations.join("\n");

        anyhow::bail!(
            "The following images exceed the maximum pixel limit of {MAX_PIXELS_PER_IMAGE}:\n{violation_messages}",
        );
    }

    Ok(())
}

fn compute_sha256_hash(path: &Path) -> Result<String> {
    use std::io::Read as _;

    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open image for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read image for hashing: {}", path.display()))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}

fn is_hidden(root: &Path, path: &Path) -> bool {
    path != root
        && path
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// Reads the companion JSON sidecar for an image, if it exists.
///
/// For an image at `path/to/button.png`, looks for `path/to/button.json`.
/// Returns a map of all key-value pairs from the JSON file.
fn read_sidecar_metadata(image_path: &Path) -> Result<HashMap<String, Value>> {
    let sidecar_path = image_path.with_extension("json");
    if !sidecar_path.is_file() {
        return Ok(HashMap::new());
    }

    debug!("Reading sidecar metadata: {}", sidecar_path.display());

    let sidecar_file = File::open(&sidecar_path)
        .with_context(|| format!("Failed to open sidecar file {}", sidecar_path.display()))?;

    serde_json::from_reader(BufReader::new(sidecar_file)).with_context(|| {
        format!(
            "Failed to read sidecar file {} as JSON",
            sidecar_path.display()
        )
    })
}

struct PreparedImage {
    path: PathBuf,
    key: String,
}

fn is_retryable(err: &Error) -> bool {
    match err {
        Error::OperationFailure { status, .. } => *status == 429 || *status == 503,
        Error::Reqwest(e) => match e.status() {
            Some(status) => status.as_u16() == 429 || status.as_u16() == 503,
            None => e.is_timeout() || e.is_connect(),
        },
        Error::Batch(inner) => is_retryable(inner),
        _ => false,
    }
}

#[derive(Default)]
struct ClassifiedResults {
    succeeded: HashSet<String>,
    fatal: Vec<(String, Error)>,
    retryable: Vec<(String, Error)>,
    unattributed: Vec<Error>,
}

fn classify_results(results: Vec<OperationResult>) -> ClassifiedResults {
    let mut classified = ClassifiedResults::default();
    for result in results {
        match result {
            OperationResult::Put(key, Ok(_)) => {
                classified.succeeded.insert(key);
            }
            OperationResult::Put(key, Err(err)) if is_retryable(&err) => {
                classified.retryable.push((key, err));
            }
            OperationResult::Put(key, Err(err)) => {
                classified.fatal.push((key, err));
            }
            OperationResult::Error(err) => {
                classified.unattributed.push(err);
            }
            _ => {}
        }
    }
    classified
}

fn upload_with_retry<F>(
    mut pending: Vec<PreparedImage>,
    mut delays: impl Iterator<Item = Duration>,
    mut send_batch: F,
) -> Vec<anyhow::Error>
where
    F: FnMut(&[PreparedImage]) -> Vec<OperationResult>,
{
    let mut failures: Vec<anyhow::Error> = Vec::new();
    let mut last_error: HashMap<String, Error> = HashMap::new();
    let mut last_unattributed: Option<Error> = None;

    loop {
        let classified = classify_results(send_batch(&pending));

        for (key, err) in classified.retryable {
            last_error.insert(key, err);
        }
        let unattributed_is_fatal = classified.unattributed.iter().any(|err| !is_retryable(err));
        if let Some(err) = classified.unattributed.into_iter().last() {
            last_unattributed = Some(err);
        }
        let fatal_keys: HashSet<String> =
            classified.fatal.iter().map(|(key, _)| key.clone()).collect();
        for (key, err) in classified.fatal {
            last_error.remove(&key);
            failures.push(anyhow::Error::new(err).context(format!("failed to upload {key}")));
        }

        pending
            .retain(|p| !classified.succeeded.contains(&p.key) && !fatal_keys.contains(&p.key));

        if pending.is_empty() || unattributed_is_fatal {
            break;
        }
        let Some(delay) = delays.next() else {
            break;
        };
        debug!(
            "{} snapshot image upload(s) pending, retrying in {} ms",
            pending.len(),
            delay.as_milliseconds()
        );
        thread::sleep(delay);
    }

    for prepared in pending {
        let err = last_error
            .remove(&prepared.key)
            .map(anyhow::Error::new)
            .or_else(|| last_unattributed.take().map(anyhow::Error::new))
            .unwrap_or_else(|| anyhow::anyhow!("operation failed after exhausting retries"));
        failures.push(err.context(format!("failed to upload {}", prepared.key)));
    }

    failures
}

fn upload_images(
    images: Vec<ImageInfo>,
    org: &str,
    project: &str,
) -> Result<HashMap<String, ImageMetadata>> {
    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    let options = authenticated_api.fetch_snapshots_upload_options(org, project)?;

    let expiration = ExpirationPolicy::from_str(&options.objectstore.expiration_policy)
        .context("Failed to parse expiration policy from upload options")?;

    let mut builder = ClientBuilder::new(options.objectstore.url);
    if let Some(token) = options.objectstore.auth_token {
        builder = builder.token(token.expose_secret().to_owned());
    }
    let client = builder
        .configure_reqwest(|r| r.connect_timeout(Duration::from_secs(10)))
        .build()?;

    let scopes = options.objectstore.scopes;

    let find_scope = |name: &str| {
        scopes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let org_id = find_scope("org").context("Missing org in UploadOptions scope")?;
    let project_id = find_scope("project").context("Missing project in UploadOptions scope")?;

    let mut scope = Usecase::new("preprod").scope();
    for (key, value) in scopes {
        scope = scope.push(&key, value);
    }

    let session = scope.session(&client)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let mut manifest_entries = HashMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    let mut uploads = Vec::with_capacity(images.len());

    let hashed_images: Vec<_> = images
        .into_par_iter()
        .map(|image| {
            let hash = compute_sha256_hash(&image.path)?;
            Ok((image, hash))
        })
        .collect::<Result<Vec<_>>>()?;

    for (image, hash) in hashed_images {
        let image_key = crate::utils::fs::path_as_url(&image.relative_path);

        if manifest_entries.contains_key(&image_key) {
            duplicates.push(image_key);
            continue;
        }

        let key = format!("{org_id}/{project_id}/{hash}");

        let mut extra = read_sidecar_metadata(&image.path).unwrap_or_else(|err| {
            warn!("Error reading sidecar metadata, ignoring it instead: {err:#}");
            HashMap::new()
        });
        extra.insert("content_hash".to_owned(), serde_json::Value::String(hash));

        uploads.push(PreparedImage {
            path: image.path,
            key,
        });
        manifest_entries.insert(
            image_key,
            ImageMetadata::new(image.width, image.height, extra),
        );
    }

    if !duplicates.is_empty() {
        let paths = duplicates.join(", ");
        warn!("Duplicate paths encountered, skipping: {paths}");
    }

    let total_count = uploads.len();

    let existing_keys: HashSet<String> = runtime.block_on(async {
        let mut head_builder = session.many();
        for prepared in &uploads {
            head_builder = head_builder.push(session.head(&prepared.key));
        }

        let mut results = head_builder.send().await;
        let mut existing = HashSet::new();
        while let Some(result) = results.next().await {
            if let OperationResult::Head(key, Ok(Some(_))) = result {
                existing.insert(key);
            }
        }
        existing
    });

    let missing_uploads: Vec<_> = uploads
        .into_iter()
        .filter(|p| !existing_keys.contains(&p.key))
        .collect();
    let skipped = total_count - missing_uploads.len();
    let upload_count = missing_uploads.len();

    if skipped > 0 {
        println!(
            "{} {} of {total_count} {} already uploaded, uploading {} new",
            style(">").dim(),
            style(skipped).yellow(),
            if total_count == 1 { "image" } else { "images" },
            style(upload_count).yellow(),
        );
    }

    if upload_count > 0 {
        let delays = get_default_backoff()
            .with_max_times(Config::current().max_retries() as usize)
            .build();

        let failures = upload_with_retry(missing_uploads, delays, |pending| {
            runtime.block_on(async {
                let mut many_builder = session.many();
                for prepared in pending {
                    many_builder = many_builder.push(
                        session
                            .put_path(prepared.path.clone())
                            .key(&prepared.key)
                            .expiration_policy(expiration),
                    );
                }

                let mut stream = many_builder.send().await;
                let mut out = Vec::new();
                while let Some(result) = stream.next().await {
                    out.push(result);
                }
                out
            })
        });

        if !failures.is_empty() {
            let error_count = failures.len();
            eprintln!("There were errors uploading images:");
            for error in failures {
                eprintln!("  {}", style(format!("{error:#}")).red());
            }
            anyhow::bail!("Failed to upload {error_count} images");
        }
    }

    println!(
        "{} Uploaded {} new image {}",
        style(">").dim(),
        style(upload_count).yellow(),
        if upload_count == 1 { "file" } else { "files" }
    );
    Ok(manifest_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_image(width: u32, height: u32) -> ImageInfo {
        ImageInfo {
            path: PathBuf::from("img.png"),
            relative_path: PathBuf::from("img.png"),
            width,
            height,
        }
    }

    #[test]
    fn test_is_retryable_operation_failure_429() {
        let err = Error::OperationFailure {
            status: 429,
            message: "rate limited".to_owned(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_operation_failure_503() {
        let err = Error::OperationFailure {
            status: 503,
            message: "unavailable".to_owned(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_not_retryable_operation_failure_404() {
        let err = Error::OperationFailure {
            status: 404,
            message: "not found".to_owned(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_batch_wrapping_429() {
        let err = Error::Batch(std::sync::Arc::new(Error::OperationFailure {
            status: 429,
            message: "rate limited".to_owned(),
        }));
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_is_not_retryable_malformed_response() {
        let err = Error::MalformedResponse("bad".to_owned());
        assert!(!is_retryable(&err));
    }

    fn put_ok(key: &str) -> OperationResult {
        OperationResult::Put(
            key.to_owned(),
            Ok(objectstore_client::PutResponse {
                key: key.to_owned(),
            }),
        )
    }

    fn put_err(key: &str, status: u16) -> OperationResult {
        OperationResult::Put(
            key.to_owned(),
            Err(Error::OperationFailure {
                status,
                message: format!("status {status}"),
            }),
        )
    }

    fn prepared(key: &str) -> PreparedImage {
        PreparedImage {
            path: PathBuf::from(format!("{key}.png")),
            key: key.to_owned(),
        }
    }

    #[test]
    fn test_classify_results_partitions_results() {
        let results = vec![
            put_ok("ok-key"),
            put_err("rl-key", 429),
            put_err("fatal-key", 404),
            OperationResult::Error(Error::MalformedResponse("bad".to_owned())),
        ];

        let classified = classify_results(results);

        assert_eq!(classified.succeeded.len(), 1);
        assert!(classified.succeeded.contains("ok-key"));
        assert_eq!(classified.fatal.len(), 1);
        assert_eq!(classified.fatal[0].0, "fatal-key");
        assert_eq!(classified.retryable.len(), 1);
        assert_eq!(classified.retryable[0].0, "rl-key");
        assert_eq!(classified.unattributed.len(), 1);
    }

    #[test]
    fn test_retry_all_succeed_first_attempt() {
        let mut attempts = 0;
        let failures = upload_with_retry(vec![prepared("a"), prepared("b")], std::iter::empty(), |p| {
            attempts += 1;
            p.iter().map(|img| put_ok(&img.key)).collect()
        });
        assert!(failures.is_empty());
        assert_eq!(attempts, 1);
    }

    #[test]
    fn test_retry_recovers_after_rate_limit() {
        let mut attempts = 0;
        let failures = upload_with_retry(vec![prepared("a")], std::iter::repeat(Duration::ZERO), |p| {
            attempts += 1;
            if attempts == 1 {
                p.iter().map(|img| put_err(&img.key, 429)).collect()
            } else {
                p.iter().map(|img| put_ok(&img.key)).collect()
            }
        });
        assert!(failures.is_empty());
        assert_eq!(attempts, 2);
    }

    #[test]
    fn test_retry_fatal_error_is_not_retried() {
        let mut attempts = 0;
        let failures = upload_with_retry(vec![prepared("a")], std::iter::repeat(Duration::ZERO), |p| {
            attempts += 1;
            p.iter().map(|img| put_err(&img.key, 404)).collect()
        });
        assert_eq!(failures.len(), 1);
        assert_eq!(attempts, 1);
    }

    #[test]
    fn test_retry_stops_on_non_retryable_unattributed_error() {
        let mut attempts = 0;
        let delays = std::iter::repeat_n(Duration::ZERO, 5);
        let failures = upload_with_retry(vec![prepared("a")], delays, |_| {
            attempts += 1;
            vec![OperationResult::Error(Error::MalformedResponse("bad".to_owned()))]
        });
        assert_eq!(attempts, 1);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_retry_exhausts_and_reports_with_real_error() {
        let mut attempts = 0;
        let delays = std::iter::repeat_n(Duration::ZERO, 2);
        let failures = upload_with_retry(vec![prepared("a")], delays, |p| {
            attempts += 1;
            p.iter().map(|img| put_err(&img.key, 429)).collect()
        });
        assert_eq!(attempts, 3);
        assert_eq!(failures.len(), 1);
        assert!(format!("{:#}", failures[0]).contains("429"));
    }

    #[test]
    fn test_validate_image_sizes_at_limit_passes() {
        assert!(validate_image_sizes(&[make_image(8000, 5000)]).is_ok());
    }

    #[test]
    fn test_validate_image_sizes_over_limit_fails() {
        let err = validate_image_sizes(&[make_image(8001, 5000)]).unwrap_err();
        assert!(err.to_string().contains("exceed the maximum pixel limit"));
    }

    #[test]
    fn test_split_and_trim_comma_separated() {
        assert_eq!(
            split_and_trim("a.png, b.png, c.png", ','),
            vec!["a.png", "b.png", "c.png"]
        );
    }

    #[test]
    fn test_split_and_trim_whitespace_and_empty() {
        assert_eq!(
            split_and_trim("  a.png , b.png ,  ", ','),
            vec!["a.png", "b.png"]
        );
    }

    #[test]
    fn test_split_and_trim_newline_separated() {
        assert_eq!(
            split_and_trim("a.png\nb.png\n\nc.png\n", '\n'),
            vec!["a.png", "b.png", "c.png"]
        );
    }

    #[test]
    fn test_normalize_image_names_strips_dot_slash() {
        let input = vec![
            "./img/a.png".to_owned(),
            "./img/b.png".to_owned(),
            "img/c.png".to_owned(),
        ];
        assert_eq!(
            normalize_image_names(input),
            vec!["img/a.png", "img/b.png", "img/c.png"]
        );
    }
}
