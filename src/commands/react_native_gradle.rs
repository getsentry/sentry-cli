use std::env;
use std::path::PathBuf;

use clap::{App, Arg, ArgMatches, AppSettings};

use prelude::*;
use config::Config;
use utils::ArgExt;
use api::{Api, NewRelease};
use sourcemaputils::SourceMapProcessor;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads react-native projects from within a gradle build step")
        // we intentionally hide this command because for all intents and purposes
        // the user should not know it exists.  It's invoked exclusively by the
        // gradle build step in react-native.
        .setting(AppSettings::Hidden)
        .org_project_args()
        .arg(Arg::with_name("sourcemap")
             .long("sourcemap")
             .value_name("PATH")
             .required(true)
             .help("The path to the sourcemap that should be uploaded"))
        .arg(Arg::with_name("bundle")
             .long("bundle")
             .value_name("PATH")
             .required(true)
             .help("The path to the bundle that should be uploaded"))
        .arg(Arg::with_name("release")
             .long("release")
             .value_name("RELEASE")
             .required(true)
             .multiple(true)
             .help("The name of the release to publish. This can be supplied \
                    multiple times."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let (org, project) = config.get_org_and_project(matches)?;
    let api = Api::new(config);
    let base = env::current_dir()?;

    let sourcemap_path = PathBuf::from(matches.value_of("sourcemap").unwrap());
    let bundle_path = PathBuf::from(matches.value_of("bundle").unwrap());
    let sourcemap_url = format!("~/{}", sourcemap_path.file_name().unwrap().to_string_lossy());
    let bundle_url = format!("~/{}", bundle_path.file_name().unwrap().to_string_lossy());

    println!("Processing react-native sourcemaps for Sentry upload.");
    info!("  bundle path: {}", bundle_path.display());
    info!("  sourcemap path: {}", sourcemap_path.display());

    let mut processor = SourceMapProcessor::new(matches.is_present("verbose"));
    processor.add(&bundle_url, &bundle_path)?;
    processor.add(&sourcemap_url, &sourcemap_path)?;
    processor.rewrite(&vec![base.parent().unwrap().to_str().unwrap()])?;
    processor.add_sourcemap_references()?;

    for version in matches.values_of("release").unwrap() {
        let release = api.new_release(&org, &NewRelease {
            version: version.to_string(),
            projects: vec![project.to_string()],
            ..Default::default()
        })?;
        println!("Uploading sourcemaps for release {}", release.version);
        processor.upload(&api, &org, Some(&project), &release.version)?;
    }

    Ok(())
}
