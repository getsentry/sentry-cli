use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt as _;
use crate::utils::fs::{path_as_url, TempFile};

const EXPERIMENTAL_WARNING: &str =
    "[EXPERIMENTAL] The \"snapshots download\" command is experimental. \
    The command is subject to breaking changes, including removal, in any Sentry CLI release.";

pub fn make_command(command: Command) -> Command {
    command
        .about("[EXPERIMENTAL] Download baseline snapshot images from Sentry.")
        .long_about(format!(
            "Download baseline snapshot images from Sentry's preprod system to a local directory.\n\n\
            Use --snapshot-id to download a specific snapshot, or --app-id to resolve the latest \
            baseline (org auth tokens require --project with a numeric project ID for --app-id).\n\n\
            {EXPERIMENTAL_WARNING}"
        ))
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("app_id")
                .long("app-id")
                .value_name("APP_ID")
                .help("App identifier (e.g. sentry-frontend). Mutually exclusive with --snapshot-id.")
                .conflicts_with("snapshot_id"),
        )
        .arg(
            Arg::new("snapshot_id")
                .long("snapshot-id")
                .value_name("ID")
                .help("Direct snapshot artifact ID. Mutually exclusive with --app-id.")
                .conflicts_with("app_id"),
        )
        .arg(
            Arg::new("branch")
                .long("branch")
                .value_name("NAME")
                .help("Git branch filter (only with --app-id).")
                .requires("app_id"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("DIR")
                .help("Directory for extracted images.")
                .default_value("./snapshots-base/"),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    eprintln!("{EXPERIMENTAL_WARNING}");

    let config = Config::current();
    let org = config.get_org(matches)?;
    let api_ref = Api::current();
    let api = api_ref.authenticated()?;

    let project = config.get_project(matches).ok();
    let app_id = matches.get_one::<String>("app_id");
    let snapshot_id_arg = matches.get_one::<String>("snapshot_id");
    let branch = matches.get_one::<String>("branch").map(|s| s.as_str());
    let output_dir = PathBuf::from(
        matches
            .get_one::<String>("output")
            .expect("output has a default value"),
    );

    let snapshot_id = match (app_id, snapshot_id_arg) {
        (Some(app_id), None) => {
            eprintln!("Resolving latest baseline snapshot for app '{app_id}'...");
            match api.get_latest_base_snapshot(&org, app_id, branch, project.as_deref())? {
                Some(resp) => {
                    eprintln!(
                        "Found snapshot {} ({} images)",
                        resp.head_artifact_id, resp.image_count
                    );
                    resp.head_artifact_id
                }
                None => {
                    let branch_msg = branch
                        .map(|b| format!(" on branch '{b}'"))
                        .unwrap_or_default();
                    bail!("No baseline snapshot found for app '{app_id}'{branch_msg}");
                }
            }
        }
        (None, Some(id)) => id.clone(),
        _ => bail!("Exactly one of --app-id or --snapshot-id must be provided"),
    };

    eprintln!("Downloading snapshot {snapshot_id}...");
    let tmp = TempFile::create()?;
    let mut tmp_file = tmp.open()?;
    let response = api.download_snapshot_zip(&org, &snapshot_id, &mut tmp_file)?;

    if response.failed() {
        bail!(
            "Failed to download snapshot (server returned status {}).",
            response.status()
        );
    }

    let mut archive = zip::ZipArchive::new(&mut tmp_file)?;

    fs::create_dir_all(&output_dir)?;

    let mut extracted = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };
        let out_path = output_dir.join(&enclosed_name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut out_file)?;
        extracted += 1;
    }

    eprintln!(
        "\nDownloaded {extracted} images from snapshot {snapshot_id} to {}",
        path_as_url(&output_dir)
    );

    Ok(())
}
