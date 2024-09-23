use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::{debug, info};
use sourcemap::ram_bundle::RamBundle;

use crate::api::Api;
use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::{validate_distribution, ArgExt};
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::UploadContext;
use crate::utils::sourcemaps::SourceMapProcessor;

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload react-native projects in a gradle build step.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("sourcemap")
                .long("sourcemap")
                .value_name("PATH")
                .required(true)
                .help("The path to a sourcemap that should be uploaded."),
        )
        .arg(
            Arg::new("bundle")
                .long("bundle")
                .value_name("PATH")
                .required(true)
                .help("The path to a bundle that should be uploaded."),
        )
        .arg(
            Arg::new("release")
                .long("release")
                .value_name("RELEASE")
                .help("The name of the release to publish."),
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
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let api = Api::current();
    let base = env::current_dir()?;

    let sourcemap_path = PathBuf::from(matches.get_one::<String>("sourcemap").unwrap());
    let bundle_path = PathBuf::from(matches.get_one::<String>("bundle").unwrap());
    let sourcemap_url = format!(
        "~/{}",
        sourcemap_path.file_name().unwrap().to_string_lossy()
    );
    let bundle_url = format!("~/{}", bundle_path.file_name().unwrap().to_string_lossy());

    info!(
        "Issuing a command for Organization: {} Project: {}",
        org, project
    );

    println!("Processing react-native sourcemaps for Sentry upload.");
    info!("  bundle path: {}", bundle_path.display());
    info!("  sourcemap path: {}", sourcemap_path.display());

    let mut processor = SourceMapProcessor::new();
    processor.add(
        &bundle_url,
        ReleaseFileSearch::collect_file(bundle_path.clone())?,
    )?;
    processor.add(
        &sourcemap_url,
        ReleaseFileSearch::collect_file(sourcemap_path)?,
    )?;

    if let Ok(ram_bundle) = RamBundle::parse_unbundle_from_path(&bundle_path) {
        debug!("File RAM bundle found, extracting its contents...");
        processor.unpack_ram_bundle(&ram_bundle, &bundle_url)?;
    } else {
        debug!("Non-file bundle found");
    }

    processor.rewrite(&[base.to_str().unwrap()])?;
    processor.add_sourcemap_references()?;
    processor.add_debug_id_references()?;

    let version = matches.get_one::<String>("release");
    let chunk_upload_options = api.authenticated()?.get_chunk_upload_options(&org)?;

    let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
    let wait = matches.get_flag("wait") || wait_for_secs.is_some();
    let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, Duration::from_secs);

    if let Some(version) = version {
        for dist in matches.get_many::<String>("dist").unwrap() {
            println!(
                "Uploading sourcemaps for release {} distribution {}",
                version, dist
            );

            processor.upload(&UploadContext {
                org: &org,
                project: Some(&project),
                release: Some(version),
                dist: Some(dist),
                note: None,
                wait,
                max_wait,
                dedupe: false,
                chunk_upload_options: chunk_upload_options.as_ref(),
            })?;
        }
    } else {
        // Debug Id Upload
        processor.upload(&UploadContext {
            org: &org,
            project: Some(&project),
            release: None,
            dist: None,
            note: None,
            wait,
            max_wait,
            dedupe: false,
            chunk_upload_options: chunk_upload_options.as_ref(),
        })?;
    }

    Ok(())
}
