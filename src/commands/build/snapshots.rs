use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::time::Duration;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use itertools::Itertools as _;
use log::{debug, warn};
use objectstore_client::{ClientBuilder, ExpirationPolicy, Usecase};
use rayon::prelude::*;
use secrecy::ExposeSecret as _;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::{Api, CreateSnapshotResponse, ImageMetadata, SnapshotsManifest};
use crate::config::{Auth, Config};
use crate::utils::args::ArgExt as _;
use crate::utils::build_vcs::collect_git_metadata;
use crate::utils::ci::is_ci;

const EXPERIMENTAL_WARNING: &str =
    "[EXPERIMENTAL] The \"build snapshots\" command is experimental. \
    The command is subject to breaking changes, including removal, in any Sentry CLI release.";

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];
const MAX_PIXELS_PER_IMAGE: u64 = 40_000_000;

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Upload build snapshots to a project.")
        .long_about(format!(
            "Upload build snapshots to a project.\n\n{EXPERIMENTAL_WARNING}"
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

    // Upload image files to objectstore
    println!(
        "{} Uploading {} image {}",
        style(">").dim(),
        style(images.len()).yellow(),
        if images.len() == 1 { "file" } else { "files" }
    );

    let manifest_entries = upload_images(images, &org, &project)?;

    // Build manifest from discovered images
    let diff_threshold = matches.get_one::<f64>("diff_threshold").copied();

    let selective = matches.get_flag("selective");

    let manifest = SnapshotsManifest {
        app_id: app_id.clone(),
        images: manifest_entries,
        diff_threshold,
        selective,
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
        builder = builder.token(token);
    }

    let sentry_token = match authenticated_api.auth() {
        Auth::Token(token) => token.raw().expose_secret().to_owned(),
    };
    let sentry_token = format!("Bearer {sentry_token}")
        .parse()
        // Ignore original error to avoid leaking the token (even though it's invalid)
        .map_err(|_| anyhow::anyhow!("Invalid auth token"))?;
    let client = builder
        .configure_reqwest(|r| {
            let mut headers = http::HeaderMap::new();
            headers.insert(http::header::AUTHORIZATION, sentry_token);
            r.connect_timeout(Duration::from_secs(10))
                .default_headers(headers)
        })
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

    let mut many_builder = session.many();
    for prepared in uploads {
        many_builder = many_builder.push(
            session
                .put_path(prepared.path.clone())
                .key(&prepared.key)
                .expiration_policy(expiration),
        );
    }

    let result = runtime.block_on(async { many_builder.send().await.error_for_failures().await });
    if let Err(errors) = result {
        let errors: Vec<_> = errors.collect();
        let error_count = errors.len();
        eprintln!("There were errors uploading images:");
        for error in errors {
            let error = anyhow::Error::new(error);
            eprintln!("  {}", style(format!("{error:#}")).red());
        }
        anyhow::bail!("Failed to upload {error_count} images");
    }

    println!(
        "{} Uploaded {} image {}",
        style(">").dim(),
        style(total_count).yellow(),
        if total_count == 1 { "file" } else { "files" }
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
    fn test_validate_image_sizes_at_limit_passes() {
        assert!(validate_image_sizes(&[make_image(8000, 5000)]).is_ok());
    }

    #[test]
    fn test_validate_image_sizes_over_limit_fails() {
        let err = validate_image_sizes(&[make_image(8001, 5000)]).unwrap_err();
        assert!(err.to_string().contains("exceed the maximum pixel limit"));
    }
}
