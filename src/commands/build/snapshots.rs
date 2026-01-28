use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use log::{debug, info};
use objectstore_client::{Client, Usecase};
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use crate::api::Api;
use crate::config::Config;
use crate::utils::api::get_org_project_id;
use crate::utils::args::ArgExt as _;
use crate::utils::objectstore::get_objectstore_url;

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
        .project_arg(false)
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

    info!("Processing build snapshots from: {path}");
    info!("Using snapshot ID: {snapshot_id}");
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
    upload_files(&files, &org, &project, snapshot_id)?;

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
) -> Result<()> {
    let url = get_objectstore_url(Api::current(), org)?;
    let client = Client::new(url)?;

    let (org, project) = get_org_project_id(Api::current(), org, project)?;
    let session = Usecase::new("preprod")
        .for_project(org, project)
        .session(&client)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let mut many_builder = session.many();

    for file_path in files {
        debug!("Processing file: {}", file_path.display());

        let contents = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

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

        info!("Queueing {} as {key}", file_path.display());

        many_builder = many_builder.push(session.put(contents).key(&key));
    }

    let upload = runtime
        .block_on(async { many_builder.send().await })
        .context("Failed to upload files")?;

    match upload.error_for_failures() {
        Ok(()) => {
            println!(
                "{} Uploaded {} {}",
                style(">").dim(),
                style(files.len()).yellow(),
                if files.len() == 1 { "file" } else { "files" }
            );
            Ok(())
        }
        Err(errors) => {
            eprintln!("There were errors uploading files:");
            for error in &errors {
                eprintln!("  {}", style(error).red());
            }
            anyhow::bail!(
                "Failed to upload {} out of {} files",
                errors.len(),
                files.len()
            )
        }
    }
}

fn compute_sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{result:x}")
}
