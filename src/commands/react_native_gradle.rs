use std::env;
use std::path::PathBuf;

use clap::{App, ArgMatches};
use failure::Error;

use api::{Api, NewRelease};
use config::Config;
use utils::args::{validate_org, validate_project};
use utils::sourcemaps::SourceMapProcessor;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Upload react-native projects in a gradle build step.")
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} "The project slug.")
        (@arg sourcemap: --sourcemap <PATH> "The path to a sourcemap that should be uploaded.")
        (@arg bundle: --bundle <PATH> "The path to a bundle that should be uploaded.")
        (@arg release: --release <RELEASE> "The name of the release to publish.")
        (@arg dist: --dist <DISTRIBUTION>... "The name(s) of the distributions to publish.")
    )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;
    let api = Api::get_current();
    let base = env::current_dir()?;

    let sourcemap_path = PathBuf::from(matches.value_of("sourcemap").unwrap());
    let bundle_path = PathBuf::from(matches.value_of("bundle").unwrap());
    let sourcemap_url = format!(
        "~/{}",
        sourcemap_path.file_name().unwrap().to_string_lossy()
    );
    let bundle_url = format!("~/{}", bundle_path.file_name().unwrap().to_string_lossy());

    println!("Processing react-native sourcemaps for Sentry upload.");
    info!("  bundle path: {}", bundle_path.display());
    info!("  sourcemap path: {}", sourcemap_path.display());

    let mut processor = SourceMapProcessor::new();
    processor.add(&bundle_url, &bundle_path)?;
    processor.add(&sourcemap_url, &sourcemap_path)?;
    processor.rewrite(&[base.to_str().unwrap()])?;
    processor.add_sourcemap_references()?;

    let release = api.new_release(
        &org,
        &NewRelease {
            version: matches.value_of("release").unwrap().to_string(),
            projects: vec![project.to_string()],
            ..Default::default()
        },
    )?;

    for dist in matches.values_of("dist").unwrap() {
        println!(
            "Uploading sourcemaps for release {} distribution {}",
            &release.version, dist
        );
        processor.upload(&api, &org, Some(&project), &release.version, Some(dist))?;
    }

    Ok(())
}
