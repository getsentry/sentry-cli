use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr as _;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use http::header::AUTHORIZATION;
use log::{debug, info, warn};
use objectstore_client::{ClientBuilder, ExpirationPolicy, Usecase};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt as _;
use http::{self, HeaderValue};

const EXPERIMENTAL_WARNING: &str =
    "[EXPERIMENTAL] The \"build snapshots\" command is experimental. \
    The command is subject to breaking changes, including removal, in any Sentry CLI release.";

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSnapshotResponse {
    artifact_id: String,
    image_count: u64,
}

// Keep in sync with https://github.com/getsentry/sentry/blob/master/src/sentry/preprod/snapshots/manifest.py
#[derive(Serialize)]
struct SnapshotsManifest {
    app_id: String,
    images: HashMap<String, ImageMetadata>,
}

// Keep in sync with https://github.com/getsentry/sentry/blob/master/src/sentry/preprod/snapshots/manifest.py
#[derive(Serialize)]
struct ImageMetadata {
    image_file_name: String,
    width: u32,
    height: u32,
}

struct ImageInfo {
    path: std::path::PathBuf,
    relative_path: String,
    width: u32,
    height: u32,
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

    debug!("Scanning for images in: {}", dir_path.display());
    debug!("Organization: {org}");
    debug!("Project: {project}");

    // Collect image files and read their dimensions
    let images = collect_images(dir_path)?;
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

    // Upload image files to objectstore
    println!(
        "{} Uploading {} image {}",
        style(">").dim(),
        style(images.len()).yellow(),
        if images.len() == 1 { "file" } else { "files" }
    );
    let manifest_entries = upload_images(images, &org, &project)?;

    // Build manifest from discovered images
    let manifest = SnapshotsManifest {
        app_id: app_id.clone(),
        images: manifest_entries,
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

    Ok(())
}

fn collect_images(dir: &Path) -> Result<Vec<ImageInfo>> {
    WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e.path()))
        .filter_map(|res| match res {
            Ok(entry) => Some(entry),
            Err(err) => {
                warn!("Failed to access file during directory walk: {err}");
                None
            }
        })
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| is_image_file(entry.path()))
        .map(|entry| collect_image_info(dir, entry.path()))
        .filter_map(|result| result.transpose())
        .collect()
}
/// Builds [`ImageInfo`] for a discovered image path during snapshot collection.
///
/// Returns `Ok(Some(ImageInfo))` when the image dimensions can be parsed,
/// `Ok(None)` when the file should be skipped (e.g. when dimensions cannot be
/// determined), and `Err` for hard failures.
fn collect_image_info(dir: &Path, path: &Path) -> Result<Option<ImageInfo>> {
    let (width, height) = match imagesize::size(path) {
        Ok(dims) => (dims.width as u32, dims.height as u32),
        Err(err) => {
            warn!("Could not read dimensions from {}: {err}", path.display());
            return Ok(None);
        }
    };
    let relative = path
        .strip_prefix(dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    Ok(Some(ImageInfo {
        path: path.to_path_buf(),
        relative_path: relative,
        width,
        height,
    }))
}

fn compute_sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{result:x}")
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
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

    let client = ClientBuilder::new(options.objectstore.url)
        .configure_reqwest(move |r| {
            let mut headers = http::HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_static("placeholder")); // TODO: get token from upload options endpoint
            r.default_headers(headers)
        })
        .build()?;

    let mut scope = Usecase::new("preprod").scope();
    for (key, value) in &options.objectstore.scopes {
        scope = scope.push(key, value);
    }
    let session = scope.session(&client)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let mut many_builder = session.many();
    let mut manifest_entries = HashMap::new();
    let image_count = images.len();

    for image in images {
        debug!("Processing image: {}", image.path.display());

        let contents = fs::read(&image.path)
            .with_context(|| format!("Failed to read image: {}", image.path.display()))?;
        let hash = compute_sha256_hash(&contents);

        info!("Queueing {} as {}", image.relative_path, hash);

        many_builder = many_builder.push(
            session
                .put(contents)
                .key(&hash)
                .expiration_policy(expiration),
        );

        let image_file_name = Path::new(&image.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        manifest_entries.insert(hash, ImageMetadata {
            image_file_name,
            width: image.width,
            height: image.height,
        });
    }

    let upload = runtime
        .block_on(async { many_builder.send().await })
        .context("Failed to upload image files")?;

    match upload.error_for_failures() {
        Ok(()) => {
            println!(
                "{} Uploaded {} image {}",
                style(">").dim(),
                style(image_count).yellow(),
                if image_count == 1 { "file" } else { "files" }
            );
            Ok(manifest_entries)
        }
        Err(errors) => {
            eprintln!("There were errors uploading images:");
            for error in &errors {
                eprintln!("  {}", style(error).red());
            }
            anyhow::bail!(
                "Failed to upload {} out of {} images",
                errors.len(),
                image_count
            )
        }
    }
}
