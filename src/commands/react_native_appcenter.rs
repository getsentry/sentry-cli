use std::env;
use std::ffi::OsStr;
use std::fs;

use clap::{App, Arg, ArgMatches};
use console::style;
use failure::Error;
use if_chain::if_chain;

use crate::api::{Api, NewRelease};
use crate::config::Config;
use crate::utils::appcenter::{get_appcenter_package, get_react_native_appcenter_release};
use crate::utils::args::ArgExt;
use crate::utils::sourcemaps::{SourceMapProcessor, UploadContext};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload react-native projects for AppCenter.")
        .org_project_args()
        .arg(
            Arg::with_name("deployment")
                .long("deployment")
                .value_name("DEPLOYMENT")
                .help("The name of the deployment. [Production, Staging]"),
        )
        .arg(
            Arg::with_name("bundle_id")
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
            Arg::with_name("version_name")
                .value_name("VERSION_NAME")
                .long("version-name")
                .help("Override version name in release name"),
        )
        .arg(
            Arg::with_name("print_release_name")
                .long("print-release-name")
                .help("Print the release name instead."),
        )
        .arg(
            Arg::with_name("app_name")
                .value_name("APP_NAME")
                .index(1)
                .required(true)
                .help("The name of the AppCenter application."),
        )
        .arg(
            Arg::with_name("platform")
                .value_name("PLATFORM")
                .index(2)
                .required(true)
                .help("The name of the app platform. [ios, android]"),
        )
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .index(3)
                .required(true)
                .multiple(true)
                .help("A list of folders with assets that should be processed."),
        )
        .arg(
            Arg::with_name("wait")
                .long("wait")
                .help("Wait for the server to fully process uploaded files."),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::current();
    let here = env::current_dir()?;
    let here_str: &str = &here.to_string_lossy();
    let (org, project) = config.get_org_and_project(matches)?;
    let app = matches.value_of("app_name").unwrap();
    let platform = matches.value_of("platform").unwrap();
    let deployment = matches.value_of("deployment").unwrap_or("Staging");
    let api = Api::current();
    let print_release_name = matches.is_present("print_release_name");

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
        matches.value_of("bundle_id"),
        matches.value_of("version_name"),
    )?;
    if print_release_name {
        println!("{}", release);
        return Ok(());
    }

    println!(
        "{} Processing react-native AppCenter sourcemaps",
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

    processor.upload(&UploadContext {
        org: &org,
        project: Some(&project),
        release: &release.version,
        dist: None,
        wait: !matches.is_present("wait"),
    })?;

    Ok(())
}
