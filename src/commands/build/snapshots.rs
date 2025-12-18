use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use log::{debug, info};
use objectstore_client::{Client, Usecase};
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::config::Config;
use crate::utils::args::ArgExt as _;

const EXPERIMENTAL_WARNING: &str =
    "[EXPERIMENTAL] The \"build snapshots\" command is experimental. \
    The command is subject to breaking changes, including removal, in any Sentry CLI release.";

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Upload build snapshots to a project.")
        .long_about(format!(
            "Upload build snapshots to a project.\n\n{EXPERIMENTAL_WARNING}"
        ))
        .org_arg()
        .project_arg(true)
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .help("The path to the folder containing build snapshots.")
                .required(true),
        )
        .arg(
            Arg::new("snapshot_id")
                .long("snapshot-id")
                .value_name("ID")
                .help("The snapshot identifier to associate with the upload.")
                .required(true),
        )
        .arg(
            Arg::new("shard_index")
                .long("shard-index")
                .value_name("INDEX")
                .value_parser(clap::value_parser!(u32))
                .default_value("0")
                .help("The shard index for this snapshot upload."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    eprintln!("{EXPERIMENTAL_WARNING}");

    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;

    let path = matches
        .get_one::<String>("path")
        .expect("path argument is required");
    let snapshot_id = matches
        .get_one::<String>("snapshot_id")
        .expect("snapshot_id argument is required");
    let shard_index = matches
        .get_one::<u32>("shard_index")
        .expect("shard_index has a default value");

    info!("Processing build snapshots from: {path}");
    info!("Using snapshot ID: {snapshot_id}");
    info!("Shard index: {shard_index}");
    info!("Organization: {org}");
    info!("Project: {project}");

    // Collect files to upload
    let files = collect_files(Path::new(path))?;

    if files.is_empty() {
        println!("{} No files found to upload", style("!").yellow());
        return Ok(());
    }

    println!(
        "{} Found {} {} to upload",
        style(">").dim(),
        style(files.len()).yellow(),
        if files.len() == 1 { "file" } else { "files" }
    );

    // Upload files using objectstore client
    upload_files(&files, &org, &project, snapshot_id, *shard_index)?;

    println!("{} Successfully uploaded snapshots", style(">").dim());
    Ok(())
}

fn collect_files(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    let mut files = Vec::new();

    if path.is_file() {
        // Only add if not hidden
        if !is_hidden_file(path) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.metadata()?.is_file() {
                let entry_path = entry.path();
                // Skip hidden files
                if !is_hidden_file(entry_path) {
                    files.push(entry_path.to_path_buf());
                }
            }
        }
    } else {
        anyhow::bail!("Path is neither a file nor directory: {}", path.display());
    }

    Ok(files)
}

fn is_hidden_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn upload_files(
    files: &[std::path::PathBuf],
    org: &str,
    project: &str,
    snapshot_id: &str,
    _shard_index: u32,
) -> Result<()> {
    // Create objectstore client
    let client = Client::new("http://127.0.0.1:8888/")?;
    let session = Usecase::new("preprod").for_project(1, 2).session(&client)?;

    // Create a multi-threaded tokio runtime for async operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    for file_path in files {
        debug!("Processing file: {}", file_path.display());

        // Read file contents
        let contents = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Determine the key based on file type
        let key = if is_json_file(file_path) {
            // For JSON files, use {org}/{snapshotId}/{filename}
            let filename = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown.json");
            format!("{org}/{snapshot_id}/{filename}")
        } else {
            // For other files, use {org}/{project}/{hash}
            let hash = compute_sha256_hash(&contents);
            format!("{org}/{project}/{hash}")
        };

        info!("Uploading {} as {key}", file_path.display());

        // Upload to objectstore using the runtime thread pool
        runtime.block_on(async {
            session
                .put(contents)
                .key(&key)
                .send()
                .await
                .with_context(|| format!("Failed to upload file: {}", file_path.display()))
        })?;

        println!(
            "{} Uploaded {} (key: {})",
            style(">").dim(),
            file_path.display(),
            style(&key).cyan()
        );
    }

    Ok(())
}

fn compute_sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{result:x}")
}
