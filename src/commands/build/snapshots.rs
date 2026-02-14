use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use http::header::AUTHORIZATION;
use log::{debug, info, warn};
use objectstore_client::{ClientBuilder, ExpirationPolicy, Usecase};
use secrecy::ExposeSecret as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::utils::api::get_org_project_id;
use crate::utils::args::ArgExt as _;
use crate::utils::objectstore::get_objectstore_url;
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

#[derive(Serialize)]
struct SnapshotsManifest {
    app_id: String,
    images: HashMap<String, ImageMetadata>,
}

#[derive(Serialize)]
struct ImageMetadata {
    file_name: String,
    width: u32,
    height: u32,
}

struct ImageInfo {
    path: std::path::PathBuf,
    relative_path: String,
    hash: String,
    width: u32,
    height: u32,
}

impl ImageInfo {
    fn into_manifest_entry(self) -> (String, ImageMetadata) {
        let file_name = Path::new(&self.relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        (
            self.hash,
            ImageMetadata {
                file_name,
                width: self.width,
                height: self.height,
            },
        )
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
    upload_images(&images, &org, &project)?;

    // Build manifest from discovered images
    let manifest = SnapshotsManifest {
        app_id: app_id.clone(),
        images: images
            .into_iter()
            .map(ImageInfo::into_manifest_entry)
            .collect(),
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
/// Returns `Ok(Some(ImageInfo))` when the file is readable and its dimensions can
/// be parsed, `Ok(None)` when the file should be skipped (currently when image
/// dimensions cannot be determined), and `Err` for hard failures such as I/O
/// errors reading the file.
fn collect_image_info(dir: &Path, path: &Path) -> Result<Option<ImageInfo>> {
    let contents = fs::read(path).with_context(|| format!("Failed to read: {}", path.display()))?;
    let (width, height) = match read_image_dimensions(&contents) {
        Some(dims) => dims,
        None => {
            warn!("Could not read dimensions from: {}", path.display());
            return Ok(None);
        }
    };
    let relative = path
        .strip_prefix(dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let hash = compute_sha256_hash(&contents);
    Ok(Some(ImageInfo {
        path: path.to_path_buf(),
        relative_path: relative,
        hash,
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

/// Read image dimensions from file bytes. Supports PNG and JPEG.
fn read_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG: signature followed by IHDR chunk with width/height
    if data.len() >= 24 && data[0..8] == *b"\x89PNG\r\n\x1a\n" {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((width, height));
    }

    // JPEG: starts with FF D8, scan for SOF marker
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        return read_jpeg_dimensions(data);
    }

    None
}

fn read_jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 1 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i + 1];
        i += 2;

        // SOF markers: C0-CF except C4 (DHT), C8 (JPG extension), and CC (DAC)
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if i + 7 < data.len() {
                let height = u16::from_be_bytes([data[i + 3], data[i + 4]]) as u32;
                let width = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                return Some((width, height));
            }
            return None;
        }

        // Skip segment using its length field
        if i + 1 < data.len() {
            let length = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
            i += length;
        } else {
            break;
        }
    }

    None
}

fn upload_images(images: &[ImageInfo], org: &str, project: &str) -> Result<()> {
    let config = Config::current();
    let auth = config
        .get_auth()
        .ok_or_else(|| anyhow::anyhow!("Authentication required"))?;
    let token = match auth {
        Auth::Token(token) => token.raw().expose_secret(),
    };

    let api = Api::current();
    let retention = api.authenticated()?.fetch_preprod_retention(org)?;
    let expiration =
        ExpirationPolicy::TimeToLive(Duration::from_secs(retention.snapshots * 24 * 60 * 60));

    let url = get_objectstore_url(Api::current(), org)?;
    let client = ClientBuilder::new(url)
        .configure_reqwest(move |r| {
            let mut headers = http::HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))
                    .expect("always a valid header value"),
            );
            r.default_headers(headers)
        })
        .build()?;

    let (org_id, project_id) = get_org_project_id(Api::current(), org, project)?;
    let session = Usecase::new("preprod")
        .for_project(org_id, project_id)
        .session(&client)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let mut many_builder = session.many();

    for image in images {
        debug!("Processing image: {}", image.path.display());

        let contents = fs::read(&image.path)
            .with_context(|| format!("Failed to read image: {}", image.path.display()))?;

        let obj_key = format!("{org_id}/{project_id}/{}", image.hash);

        info!("Queueing {} as {obj_key}", image.path.display());

        many_builder = many_builder.push(
            session
                .put(contents)
                .key(&obj_key)
                .expiration_policy(expiration),
        );
    }

    let upload = runtime
        .block_on(async { many_builder.send().await })
        .context("Failed to upload image files")?;

    match upload.error_for_failures() {
        Ok(()) => {
            println!(
                "{} Uploaded {} image {}",
                style(">").dim(),
                style(images.len()).yellow(),
                if images.len() == 1 { "file" } else { "files" }
            );
            Ok(())
        }
        Err(errors) => {
            eprintln!("There were errors uploading images:");
            for error in &errors {
                eprintln!("  {}", style(error).red());
            }
            anyhow::bail!(
                "Failed to upload {} out of {} images",
                errors.len(),
                images.len()
            )
        }
    }
}
