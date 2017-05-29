use std::fs;
use std::env;
use std::ffi::OsStr;

use clap::{App, Arg, ArgMatches};
use console::style;

use prelude::*;
use api::{Api, NewRelease};
use config::Config;
use utils::ArgExt;
use codepush::{get_codepush_package, get_codepush_release};
use utils::SourceMapProcessor;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads react-native projects for codepush")
        .org_project_args()
        .arg(Arg::with_name("deployment")
            .long("deployment")
            .value_name("DEPLOYMENT")
            .help("The name of the deployment (Production, Staging)"))
        .arg(Arg::with_name("app_name")
            .value_name("APP_NAME")
            .index(1)
            .required(true)
            .help("The name of the code-push application"))
        .arg(Arg::with_name("platform")
            .value_name("PLATFORM")
            .index(2)
            .required(true)
            .help("The name of the code-push platform (ios, android)"))
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .index(3)
            .required(true)
            .multiple(true)
            .help("A list of folders with assets that should be processed."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let here = env::current_dir()?;
    let here_str: &str = &here.to_string_lossy();
    let (org, project) = config.get_org_and_project(matches)?;
    let app = matches.value_of("app_name").unwrap();
    let platform = matches.value_of("platform").unwrap();
    let deployment = matches.value_of("deployment").unwrap_or("Staging");
    let api = Api::new(config);

    println!("{} Fetching latest code-push package info", style(">").dim());
    let package = get_codepush_package(app, deployment)?;
    let release = get_codepush_release(&package, platform)?;

    println!("{} Processing react-native code-push sourcemaps",
             style(">").dim());

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

    processor.rewrite(&vec![here_str])?;
    processor.add_sourcemap_references()?;

    let release = api.new_release(&org, &NewRelease {
        version: release.to_string(),
        projects: vec![project.to_string()],
        ..Default::default()
    })?;
    processor.upload(&api, &org, Some(&project), &release.version, None)?;

    Ok(())
}
