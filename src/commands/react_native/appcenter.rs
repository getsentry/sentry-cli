#![expect(clippy::unwrap_used, reason = "deprecated command")]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;
use if_chain::if_chain;

use crate::api::Api;
use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::appcenter::{get_appcenter_package, get_react_native_appcenter_release};
use crate::utils::args::{validate_distribution, ArgExt};
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::UploadContext;
use crate::utils::sourcemaps::SourceMapProcessor;

pub fn make_command(command: Command) -> Command {
    command
        .about("[DEPRECATED] Upload react-native projects for AppCenter.")
        .hide(true)
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("deployment")
                .long("deployment")
                .value_name("DEPLOYMENT")
                .help("The name of the deployment. [Production, Staging]"),
        )
        .arg(
            Arg::new("bundle_id")
                .value_name("BUNDLE_ID")
                .long("bundle-id")
                .help(
                    "Explicitly provide the bundle ID instead of \
                     parsing the source projects.  This allows you to push \
                     codepush releases for iOS on platforms without Xcode or \
                     codepush releases for Android when you use different \
                     bundle IDs for release and debug etc.",
                ),
        )
        .arg(
            Arg::new("version_name")
                .value_name("VERSION_NAME")
                .long("version-name")
                .help("Override version name in release name"),
        )
        .arg(
            Arg::new("dist")
                .long("dist")
                .value_name("DISTRIBUTION")
                .action(ArgAction::Append)
                .value_parser(validate_distribution)
                .help("The names of the distributions to publish. Can be supplied multiple times."),
        )
        .arg(
            Arg::new("print_release_name")
                .long("print-release-name")
                .action(ArgAction::SetTrue)
                .help("Print the release name instead."),
        )
        .arg(
            Arg::new("release_name")
                .value_name("RELEASE_NAME")
                .long("release-name")
                .conflicts_with_all(["bundle_id", "version_name"])
                .help("Override the entire release-name"),
        )
        .arg(
            Arg::new("app_name")
                .value_name("APP_NAME")
                .required(true)
                .help("The name of the AppCenter application."),
        )
        .arg(
            Arg::new("platform")
                .value_name("PLATFORM")
                .required(true)
                .help("The name of the app platform. [ios, android]"),
        )
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .required(true)
                .num_args(1..)
                .action(ArgAction::Append)
                .help("A list of folders with assets that should be processed."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .conflicts_with("wait_for")
                .help("Wait for the server to fully process uploaded files."),
        )
        .arg(
            Arg::new("wait_for")
                .long("wait-for")
                .value_name("SECS")
                .value_parser(clap::value_parser!(u64))
                .conflicts_with("wait")
                .help(
                    "Wait for the server to fully process uploaded files, \
                     but at most for the given number of seconds.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    eprintln!("{}", style("⚠ DEPRECATION NOTICE: This functionality will be removed in a future version of `sentry-cli`. \
        Use the `sourcemaps upload` command instead.").yellow());

    let config = Config::current();
    let here = env::current_dir()?;
    let here_str: &str = &here.to_string_lossy();
    let org = config.get_org(matches)?;
    let projects = config.get_projects(matches)?;
    let app = matches.get_one::<String>("app_name").unwrap();
    let platform = matches.get_one::<String>("platform").unwrap();
    let deployment = matches
        .get_one::<String>("deployment")
        .map(String::as_str)
        .unwrap_or("Staging");
    let api = Api::current();
    let print_release_name = matches.get_flag("print_release_name");

    if !print_release_name {
        println!(
            "{} Fetching latest AppCenter deployment info",
            style(">").dim()
        );
    }

    let package = get_appcenter_package(app, deployment)?;
    let release = get_react_native_appcenter_release(
        &package,
        platform,
        matches.get_one::<String>("bundle_id").map(String::as_str),
        matches
            .get_one::<String>("version_name")
            .map(String::as_str),
        matches
            .get_one::<String>("release_name")
            .map(String::as_str),
    )?;
    if print_release_name {
        println!("{release}");
        return Ok(());
    }

    println!(
        "{} Processing react-native AppCenter sourcemaps",
        style(">").dim()
    );

    let mut processor = SourceMapProcessor::new();

    for path in matches.get_many::<String>("paths").unwrap() {
        let entries = fs::read_dir(path)
            .map_err(|e| anyhow!(e).context(format!("Failed processing path: \"{}\"", &path)))?;

        for entry in entries.flatten() {
            if_chain! {
                if let Some(filename) = entry.file_name().to_str();
                if let Some(ext) = entry.path().extension();
                if ext == OsStr::new("jsbundle") ||
                   ext == OsStr::new("map") ||
                   ext == OsStr::new("bundle");
                then {
                    let url = format!("~/{filename}");
                    processor.add(&url, ReleaseFileSearch::collect_file(entry.path())?);
                }
            }
        }
    }

    processor.rewrite(&[here_str])?;
    processor.add_sourcemap_references();

    let chunk_upload_options = api.authenticated()?.get_chunk_upload_options(&org)?;

    let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
    let wait = matches.get_flag("wait") || wait_for_secs.is_some();
    let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, Duration::from_secs);

    match matches.get_many::<String>("dist") {
        None => {
            println!(
                "Uploading sourcemaps for release {} (no distribution value given; use --dist to set distribution value)",
                &release
            );

            processor.upload(&UploadContext {
                org: &org,
                projects: &projects,
                release: Some(&release),
                dist: None,
                note: None,
                wait,
                max_wait,
                dedupe: false,
                chunk_upload_options: chunk_upload_options.as_ref(),
            })?;
        }
        Some(dists) => {
            for dist in dists {
                println!(
                    "Uploading sourcemaps for release {} distribution {}",
                    &release, dist
                );

                processor.upload(&UploadContext {
                    org: &org,
                    projects: &projects,
                    release: Some(&release),
                    dist: Some(dist),
                    note: None,
                    wait,
                    max_wait,
                    dedupe: false,
                    chunk_upload_options: chunk_upload_options.as_ref(),
                })?;
            }
        }
    }

    Ok(())
}
