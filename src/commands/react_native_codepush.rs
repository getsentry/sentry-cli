use std::env;
use std::ffi::OsStr;
use std::fs;

use clap::{App, AppSettings, ArgMatches};
use console::style;
use failure::Error;

use api::{Api, NewRelease};
use config::Config;
use utils::args::{validate_org, validate_project};
use utils::codepush::{get_codepush_package, get_react_native_codepush_release};
use utils::sourcemaps::SourceMapProcessor;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "DEPRECATED: Upload react-native projects for CodePush.")
        (setting: AppSettings::Hidden)
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} "The project slug.")
        (@arg deployment: --deployment [DEPLOYMENT]
            "The name of the deployment. [Production, Staging]")
        (@arg bundle_id: --("bundle-id") [BUNDLE_ID]
            "Explicitly provide the bundle ID instead of \
             parsing the source projects.  This allows you to push \
             codepush releases for iOS on platforms without Xcode or \
             codepush releases for Android when you use different \
             bundle IDs for release and debug etc.")
        (@arg print_release_name: --("print-release-name") "Print the release name instead.")
        (@arg app_name: <APP_NAME> "The name of the CodePush application.")
        (@arg platform: <PLATFORM> "The name of the CodePush platform. [ios, android]")
        (@arg paths: <PATH>... "A list of folders with assets that should be processed.")
    )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::get_current();
    let here = env::current_dir()?;
    let here_str: &str = &here.to_string_lossy();
    let (org, project) = config.get_org_and_project(matches)?;
    let app = matches.value_of("app_name").unwrap();
    let platform = matches.value_of("platform").unwrap();
    let deployment = matches.value_of("deployment").unwrap_or("Staging");
    let api = Api::get_current();
    let print_release_name = matches.is_present("print_release_name");

    if !print_release_name {
        println!(
            "{} Fetching latest code-push package info",
            style(">").dim()
        );
    }

    let package = get_codepush_package(app, deployment)?;
    let release =
        get_react_native_codepush_release(&package, platform, matches.value_of("bundle_id"))?;
    if print_release_name {
        println!("{}", release);
        return Ok(());
    }

    println!(
        "{} Processing react-native code-push sourcemaps",
        style(">").dim()
    );

    let mut processor = SourceMapProcessor::new();
    for path in matches.values_of("paths").unwrap() {
        for entry in fs::read_dir(path)? {
            if_chain! {
                if let Ok(entry) = entry;
                if let Some(filename) = entry.file_name().to_str();
                if let Some(ext) = entry.path().extension();
                if ext == OsStr::new("jsbundle") ||
                   ext == OsStr::new("map") ||
                   ext == OsStr::new("bundle");
                then {
                    let url = format!("~/{}", filename);
                    processor.add(&url, &entry.path())?;
                }
            }
        }
    }

    processor.rewrite(&[here_str])?;
    processor.add_sourcemap_references()?;

    let release = api.new_release(
        &org,
        &NewRelease {
            version: release.to_string(),
            projects: vec![project.to_string()],
            ..Default::default()
        },
    )?;
    processor.upload(&api, &org, Some(&project), &release.version, None)?;

    Ok(())
}
