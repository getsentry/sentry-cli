//! Implements a command for managing releases.
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::collections::HashSet;

use clap::{App, AppSettings, Arg, ArgMatches};
use walkdir::WalkDir;

use CliResult;
use api::{Api, NewRelease};
use config::Config;
use utils::make_subcommand;
use sourcemaps::SourceMapValidator;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("manage releases on Sentry")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("org")
             .value_name("ORG")
             .long("org")
             .short("o")
             .help("The organization slug"))
        .arg(Arg::with_name("project")
             .value_name("PROJECT")
             .long("project")
             .short("p")
             .help("The project slug"))
        .subcommand(make_subcommand("new")
            .about("Create a new release")
            .arg(Arg::with_name("version")
                 .value_name("VERSION")
                 .required(true)
                 .index(1)
                 .help("The version identifier for this release"))
            .arg(Arg::with_name("ref")
                 .long("ref")
                 .value_name("REF")
                 .help("Optional commit reference (commit hash)"))
            .arg(Arg::with_name("url")
                 .long("url")
                 .value_name("URL")
                 .help("Optional URL to the release for information purposes")))
        .subcommand(make_subcommand("delete")
            .about("Delete a release")
            .arg(Arg::with_name("version")
                 .value_name("VERSION")
                 .required(true)
                 .index(1)
                 .help("The version to delete")))
        .subcommand(make_subcommand("list")
            .about("list the most recent releases"))
        .subcommand(make_subcommand("files")
            .about("manage release artifact files")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .arg(Arg::with_name("version")
                 .value_name("VERSION")
                 .required(true)
                 .index(1)
                 .help("The release to manage the files of"))
            .subcommand(make_subcommand("list")
                .about("List all release files"))
            .subcommand(make_subcommand("delete")
                .about("Delete a release file")
                .arg(Arg::with_name("all")
                     .short("A")
                     .long("all")
                     .help("deletes all files"))
                .arg(Arg::with_name("names")
                     .value_name("NAMES")
                     .index(1)
                     .multiple(true)
                     .help("a list of filenames to delete.")))
            .subcommand(make_subcommand("upload")
                .about("Uploads a file for a given release")
                .arg(Arg::with_name("path")
                     .value_name("PATH")
                     .index(1)
                     .required(true)
                     .help("The file to upload"))
                .arg(Arg::with_name("name")
                     .index(2)
                     .value_name("NAME")
                     .help("The name of the file on the server.")))
            .subcommand(make_subcommand("upload-sourcemaps")
                .about("Uploads sourcemap information for a given release")
                .arg(Arg::with_name("paths")
                     .value_name("PATHS")
                     .index(1)
                     .required(true)
                     .multiple(true)
                     .help("The files to upload"))
                .arg(Arg::with_name("url_prefix")
                     .short("u")
                     .long("url-prefix")
                     .value_name("PREFIX")
                     .help("The URL prefix to prepend to all filenames"))
                .arg(Arg::with_name("validate")
                     .long("validate")
                     .help("Enable basic sourcemap validation"))
                .arg(Arg::with_name("verbose")
                     .long("verbose")
                     .short("verbose")
                     .help("Enable verbose mode"))
                .arg(Arg::with_name("extensions")
                     .long("ext")
                     .short("x")
                     .value_name("EXT")
                     .multiple(true)
                     .help("Add a file extension to the list of files to upload."))))
}

fn execute_new<'a>(matches: &ArgMatches<'a>, config: &Config,
                   org: &str, project: &str) -> CliResult<()> {
    let info_rv = Api::new(config).new_release(org, project, &NewRelease {
        version: matches.value_of("version").unwrap().to_owned(),
        reference: matches.value_of("ref").map(|x| x.to_owned()),
        url: matches.value_of("url").map(|x| x.to_owned()),
    })?;
    println!("Created release {}.", info_rv.version);
    Ok(())
}

fn execute_delete<'a>(matches: &ArgMatches<'a>, config: &Config,
                      org: &str, project: &str) -> CliResult<()> {
    let version = matches.value_of("version").unwrap();
    if Api::new(config).delete_release(org, project, version)? {
        println!("Deleted release {}!", version);
    } else {
        println!("Did nothing. Release with this version ({}) does not exist.", version);
    }
    Ok(())
}

fn execute_list<'a>(_matches: &ArgMatches<'a>, config: &Config,
                    org: &str, project: &str) -> CliResult<()> {
    for info in Api::new(config).list_releases(org, project)? {
        println!("[{}] {}: {} ({} new groups)",
                 info.date_released.unwrap_or("              unreleased".into()),
                 info.version,
                 info.reference.unwrap_or("-".into()),
                 info.new_groups);
    }
    Ok(())
}

fn execute_files_list<'a>(_matches: &ArgMatches<'a>, config: &Config,
                          org: &str, project: &str, release: &str) -> CliResult<()> {
    for artifact in Api::new(config).list_release_files(org, project, release)? {
        println!("{}  ({} bytes)", artifact.name, artifact.size);
    }
    Ok(())
}

fn execute_files_delete<'a>(matches: &ArgMatches<'a>, config: &Config,
                            org: &str, project: &str, release: &str) -> CliResult<()> {
    let files : HashSet<String> = match matches.values_of("names") {
        Some(paths) => paths.map(|x| x.into()).collect(),
        None => HashSet::new(),
    };
    let api = Api::new(config);
    for file in api.list_release_files(org, project, release)? {
        if !(matches.is_present("all") || files.contains(&file.name)) {
            continue;
        }
        if api.delete_release_file(org, project, release, &file.id)? {
            println!("D {}", file.name);
        }
    }
    Ok(())
}

fn execute_files_upload<'a>(matches: &ArgMatches<'a>, config: &Config,
                            org: &str, project: &str, version: &str) -> CliResult<()> {
    let path = Path::new(matches.value_of("path").unwrap());
    let name = match matches.value_of("name") {
        Some(name) => name,
        None => Path::new(path).file_name()
            .and_then(|x| x.to_str()).ok_or("No filename provided.")?,
    };
    if let Some(artifact) = Api::new(config).upload_release_file(
        org, project, &version, &path, &name)? {
        println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
    } else {
        fail!("File already present!");
    }
    Ok(())
}

fn execute_files_upload_sourcemaps<'a>(matches: &ArgMatches<'a>, config: &Config,
                                       org: &str, project: &str, version: &str) -> CliResult<()> {
    let api = Api::new(config);
    let release = api.get_release(org, project, version)?.ok_or("release not found")?;
    let url_prefix = matches.value_of("url_prefix").unwrap_or("~").trim_right_matches("/");
    let paths = matches.values_of("paths").unwrap();
    let extensions = match matches.values_of("extensions") {
        Some(matches) => matches.map(|ext| OsStr::new(ext.trim_left_matches("."))).collect(),
        None => vec![OsStr::new("js"), OsStr::new("map")],
    };

    let mut to_process = vec![];

    for path in paths {
        // if we start walking over something that is an actual file then
        // the directory iterator yields that path and terminates.  We
        // handle that case here specifically to figure out what the path is
        // we should strip off.
        let walk_path = PathBuf::from(&path);
        let (base_path, skip_ext_test) = if walk_path.is_file() {
            (walk_path.parent().unwrap(), true)
        } else {
            (walk_path.as_path(), false)
        };

        for dent in WalkDir::new(&walk_path) {
            let dent = dent?;
            if !skip_ext_test {
                let extension = dent.path().extension();
                if !extensions.iter().any(|ext| Some(*ext) == extension) {
                    continue;
                }
            }
            let local_path = dent.path().strip_prefix(&base_path).unwrap();
            let url = format!("{}/{}", url_prefix, local_path.display());
            to_process.push((url, local_path.to_path_buf(),
                             dent.path().to_path_buf()));
        }
    }

    if matches.is_present("validate") {
        println!("Running with sourcemap validation");
        let mut validator = SourceMapValidator::new(matches.is_present("verbose"));
        for &(ref url, _, ref path) in to_process.iter() {
            validator.consider_file(path, url);
        }
        validator.validate_sources()?;
    }

    println!("Uploading sourcemaps for release {}", release.version);
    for (url, local_path, path) in to_process {
        println!("{} -> {}", local_path.display(), url);
        if let Some(artifact) = api.upload_release_file(
            org, project, &release.version, &path, &url)? {
            println!("  {}  ({} bytes)", artifact.sha1, artifact.size);
        } else {
            println!("  already present");
        }
    }

    Ok(())
}

fn execute_files<'a>(matches: &ArgMatches<'a>, config: &Config,
                     org: &str, project: &str) -> CliResult<()> {
    let release = matches.value_of("version").unwrap();
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_files_list(sub_matches, config, org, project, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        return execute_files_delete(sub_matches, config, org, project, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("upload") {
        return execute_files_upload(sub_matches, config, org, project, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("upload-sourcemaps") {
        return execute_files_upload_sourcemaps(sub_matches, config, org, project, release);
    }
    unreachable!();
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_new(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_delete(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_list(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("files") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_files(sub_matches, config, &org, &project);
    }
    unreachable!();
}
