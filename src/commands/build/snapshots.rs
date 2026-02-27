use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use log::{debug, info, warn};
use objectstore_client::{ClientBuilder, ExpirationPolicy, Usecase};
use secrecy::ExposeSecret as _;
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::{
    Api, CreateSnapshotResponse, ImageMetadata, SnapshotManifestFile, SnapshotsManifest,
};
use crate::config::{Auth, Config};
use crate::utils::args::ArgExt as _;

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

struct ImageInfo {
    path: PathBuf,
    relative_path: PathBuf,
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

    // Upload image files to objectstore
    println!(
        "{} Uploading {} image {}",
        style(">").dim(),
        style(images.len()).yellow(),
        if images.len() == 1 { "file" } else { "files" }
    );
    let mut manifest_entries = upload_images(images, &org, &project)?;

    // Parse JSON manifest files and merge metadata into discovered images
    let json_manifests = collect_manifests(dir_path);
    if !json_manifests.is_empty() {
        merge_manifest_metadata(&mut manifest_entries, &json_manifests);
    }

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

fn compute_sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{result:x}")
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
        .token({
            // TODO: replace with auth from `ObjectstoreUploadOptions` when appropriate
            let auth = match authenticated_api.auth() {
                Auth::Token(token) => token.raw().expose_secret().to_owned(),
            };
            auth
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

        info!("Queueing {} as {hash}", image.relative_path.display());

        many_builder = many_builder.push(
            session
                .put(contents)
                .key(&hash)
                .expiration_policy(expiration),
        );

        let image_file_name = image
            .relative_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        manifest_entries.insert(
            hash,
            ImageMetadata {
                image_file_name,
                width: image.width,
                height: image.height,
                display_name: None,
            },
        );
    }

    let result = runtime.block_on(async { many_builder.send().error_for_failures().await });

    match result {
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
            let mut error_count = 0;
            for error in errors {
                eprintln!("  {}", style(error).red());
                error_count += 1;
            }
            anyhow::bail!("Failed to upload {error_count} out of {image_count} images")
        }
    }
}

fn collect_manifests(dir: &Path) -> Vec<SnapshotManifestFile> {
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
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let path = entry.path();
            debug!("Reading manifest file: {}", path.display());
            let contents = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(err) => {
                    warn!("Failed to read manifest file {}: {err}", path.display());
                    return None;
                }
            };
            match serde_json::from_str::<SnapshotManifestFile>(&contents) {
                Ok(manifest) => Some(manifest),
                Err(err) => {
                    warn!("Failed to parse manifest file {}: {err}", path.display());
                    None
                }
            }
        })
        .collect()
}

fn merge_manifest_metadata(
    manifest_entries: &mut HashMap<String, ImageMetadata>,
    json_manifests: &[SnapshotManifestFile],
) {
    for json_manifest in json_manifests {
        for json_image in json_manifest.images.values() {
            let matched = manifest_entries
                .values_mut()
                .find(|entry| entry.image_file_name == json_image.image_file_name);
            match matched {
                Some(entry) => {
                    if let Some(ref display_name) = json_image.display_name {
                        debug!(
                            "Setting display_name for {}: {display_name}",
                            entry.image_file_name
                        );
                        entry.display_name = Some(display_name.clone());
                    }
                }
                None => {
                    warn!(
                        "Manifest entry for '{}' does not match any discovered image",
                        json_image.image_file_name
                    );
                }
            }
        }
    }
}
