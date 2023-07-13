use std::env;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::{debug, info};
use sourcemap::ram_bundle::RamBundle;

use crate::api::Api;
use crate::config::Config;
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
                .required(true)
                .help("The name of the release to publish."),
        )
        .arg(
            Arg::new("dist")
                .long("dist")
                .value_name("DISTRIBUTION")
                .required(true)
                .action(ArgAction::Append)
                .value_parser(validate_distribution)
                .help("The names of the distributions to publish. Can be supplied multiple times."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .help("Wait for the server to fully process uploaded files."),
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

    // TODO: make this optional
    let version = matches.get_one::<String>("release").unwrap().to_string();
    let chunk_upload_options = api.get_chunk_upload_options(&org)?;

    for dist in matches.get_many::<String>("dist").unwrap() {
        println!(
            "Uploading sourcemaps for release {} distribution {}",
            &version, dist
        );

        processor.upload(&UploadContext {
            org: &org,
            project: Some(&project),
            release: Some(&version),
            dist: Some(dist),
            note: None,
            wait: matches.get_flag("wait"),
            dedupe: false,
            chunk_upload_options: chunk_upload_options.as_ref(),
        })?;
    }

    Ok(())
}
