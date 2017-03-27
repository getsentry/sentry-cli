//! Implements a command for managing releases.
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::collections::HashSet;
use std::fmt;

use clap::{App, AppSettings, Arg, ArgMatches};
use walkdir::WalkDir;
use chrono::{DateTime, UTC};
use regex::Regex;

use prelude::*;
use api::{Api, NewRelease, UpdatedRelease, FileContents};
use config::Config;
use sourcemaputils::SourceMapProcessor;
use utils::ArgExt;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("manage releases on Sentry")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_project_args()
        .subcommand(App::new("new")
            .about("Create a new release")
            .version_arg(1)
            .arg(Arg::with_name("ref")
                .long("ref")
                .value_name("REF")
                .help("Optional commit reference (commit hash)"))
            .arg(Arg::with_name("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes"))
            .arg(Arg::with_name("finalize")
                 .long("finalize")
                 .help("Immediately finalize the release (sets it to released)")))
        .subcommand(App::new("delete")
            .about("Delete a release")
            .version_arg(1))
        .subcommand(App::new("finalize")
            .about("Marks a release as finalized and released.")
            .version_arg(1)
            .arg(Arg::with_name("started")
                 .long("started")
                 .value_name("DATE")
                 .help("If set the release start date is set to this value."))
            .arg(Arg::with_name("released")
                 .long("released")
                 .value_name("DATE")))
        .subcommand(App::new("list").about("list the most recent releases"))
        .subcommand(App::new("files")
            .about("manage release artifact files")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .version_arg(1)
            .subcommand(App::new("list").about("List all release files"))
            .subcommand(App::new("delete")
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
            .subcommand(App::new("upload")
                .about("Uploads a file for a given release")
                .arg(Arg::with_name("headers")
                    .long("header")
                    .short("H")
                    .value_name("KEY VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Stores a header with this file"))
                .arg(Arg::with_name("path")
                    .value_name("PATH")
                    .index(1)
                    .required(true)
                    .help("The file to upload"))
                .arg(Arg::with_name("name")
                    .index(2)
                    .value_name("NAME")
                    .help("The name of the file on the server.")))
            .subcommand(App::new("upload-sourcemaps")
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
                .arg(Arg::with_name("no_sourcemap_reference")
                    .long("no-sourcemap-reference")
                    .help("Disables the emitting of automatic sourcemap references. \
                            By default the tool will store a 'Sourcemap' header with \
                            minified files so that sourcemaps are located automatically \
                            if the tool can detect a link. If this causes issues it can \
                            be disabled."))
                .arg(Arg::with_name("rewrite")
                    .long("rewrite")
                    .help("Enables rewriting of matching sourcemaps \
                            so that indexed maps are flattened and missing \
                            sources are inlined if possible.  This fundamentally \
                            changes the upload process to be based on sourcemaps \
                            and minified files exclusively and comes in handy for \
                            setups like react-native that generate sourcemaps that \
                            would otherwise not work for sentry."))
                .arg(Arg::with_name("strip_prefix")
                    .long("strip-prefix")
                    .value_name("PREFIX")
                    .multiple(true)
                    .help("When passed all sources that start with the given prefix \
                            will have that prefix stripped from the filename.  This \
                            requires --rewrite to be enabled."))
                .arg(Arg::with_name("strip_common_prefix")
                    .long("strip-common-prefix")
                    .help("Similar to --strip-prefix but strips the most common \
                            prefix on all sources."))
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

fn strip_version(version: &str) -> &str {
    lazy_static! {
        static ref SHA_RE: Regex = Regex::new(r"^[a-fA-F0-9]{40}$").unwrap();
    }
    if SHA_RE.is_match(version) {
        &version[..12]
    } else {
        version
    }
}

#[cfg(windows)]
fn path_as_url(path: &Path) -> String {
    path.display().to_string().replace("\\", "/")
}

#[cfg(not(windows))]
fn path_as_url(path: &Path) -> String {
    path.display().to_string()
}

fn execute_new<'a>(matches: &ArgMatches<'a>,
                   config: &Config,
                   org: &str,
                   project: &str)
                   -> Result<()> {
    let info_rv = Api::new(config).new_release(org,
        project,
        &NewRelease {
            version: matches.value_of("version").unwrap().to_owned(),
            reference: matches.value_of("ref").map(|x| x.to_owned()),
            url: matches.value_of("url").map(|x| x.to_owned()),
            date_started: Some(UTC::now()),
            date_released: if matches.is_present("finalize") {
                Some(UTC::now())
            } else {
                None
            },
            ..Default::default()
        })?;
    println!("Created release {}.", info_rv.version);
    Ok(())
}

fn execute_finalize<'a>(matches: &ArgMatches<'a>,
                        config: &Config,
                        org: &str,
                        project: &str)
                        -> Result<()> {
    fn get_date(value: Option<&str>, now_default: bool) -> Result<Option<DateTime<UTC>>> {
        match value {
            None => Ok(if now_default { Some(UTC::now()) } else { None }),
            Some(value) => Ok(Some(value.parse().chain_err(
                || Error::from("Invalid date format."))?))
        }
    }

    let info_rv = Api::new(config).update_release(org,
        project,
        matches.value_of("version").unwrap(),
        &UpdatedRelease {
            reference: matches.value_of("ref").map(|x| x.to_owned()),
            url: matches.value_of("url").map(|x| x.to_owned()),
            date_started: get_date(matches.value_of("started"), false)?,
            date_released: get_date(matches.value_of("released"), true)?,
            ..Default::default()
        })?;
    println!("Finalized release {}.", info_rv.version);
    Ok(())
}

fn execute_delete<'a>(matches: &ArgMatches<'a>,
                      config: &Config,
                      org: &str,
                      project: &str)
                      -> Result<()> {
    let version = matches.value_of("version").unwrap();
    if Api::new(config).delete_release(org, project, version)? {
        println!("Deleted release {}!", version);
    } else {
        println!("Did nothing. Release with this version ({}) does not exist.",
                 version);
    }
    Ok(())
}

fn execute_list<'a>(_matches: &ArgMatches<'a>,
                    config: &Config,
                    org: &str,
                    project: &str)
                    -> Result<()> {
    let releases = Api::new(config).list_releases(org, project)?;
    println!("RELEASED                     VERSION       NEW EVENTS");
    println!("=====================================================");
    for info in releases {
        println!("{: ^27}  {: <12}  {}",
                 info.date_released.as_ref()
                    .map(|x| x as &fmt::Display)
                    .unwrap_or(&"(unreleased)" as &fmt::Display),
                 strip_version(&info.version),
                 info.new_groups);
    }
    Ok(())
}

fn execute_files_list<'a>(_matches: &ArgMatches<'a>,
                          config: &Config,
                          org: &str,
                          project: &str,
                          release: &str)
                          -> Result<()> {
    for artifact in Api::new(config).list_release_files(org, project, release)? {
        print!("{}  ({} bytes)", artifact.name, artifact.size);
        if let Some(sm_ref) = artifact.get_sourcemap_reference() {
            print!("  -> sourcemap: {}", sm_ref);
        }
        println!("");
    }
    Ok(())
}

fn execute_files_delete<'a>(matches: &ArgMatches<'a>,
                            config: &Config,
                            org: &str,
                            project: &str,
                            release: &str)
                            -> Result<()> {
    let files: HashSet<String> = match matches.values_of("names") {
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

fn execute_files_upload<'a>(matches: &ArgMatches<'a>,
                            config: &Config,
                            org: &str,
                            project: &str,
                            version: &str)
                            -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());
    let name = match matches.value_of("name") {
        Some(name) => name,
        None => {
            Path::new(path).file_name()
                .and_then(|x| x.to_str())
                .ok_or("No filename provided.")?
        }
    };
    let mut headers = vec![];
    if let Some(header_list) = matches.values_of("header") {
        for header in header_list {
            if !header.contains(':') {
                fail!("Invalid header. Needs to be in key:value format");
            }
            let mut iter = header.splitn(2, ':');
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    };
    if let Some(artifact) = Api::new(config).upload_release_file(org,
                             project,
                             &version,
                             FileContents::FromPath(&path),
                             &name,
                             Some(&headers[..]))? {
        println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
    } else {
        fail!("File already present!");
    }
    Ok(())
}

fn execute_files_upload_sourcemaps<'a>(matches: &ArgMatches<'a>,
                                       config: &Config,
                                       org: &str,
                                       project: &str,
                                       version: &str)
                                       -> Result<()> {
    let api = Api::new(config);

    let url_prefix = matches.value_of("url_prefix").unwrap_or("~").trim_right_matches("/");
    let paths = matches.values_of("paths").unwrap();
    let extensions = match matches.values_of("extensions") {
        Some(matches) => matches.map(|ext| OsStr::new(ext.trim_left_matches("."))).collect(),
        None => {
            vec![OsStr::new("js"), OsStr::new("map"), OsStr::new("jsbundle"), OsStr::new("bundle")]
        }
    };

    let mut processor = SourceMapProcessor::new(matches.is_present("verbose"));

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
            debug!("found: {} ({} bytes)",
                   dent.path().display(),
                   dent.metadata().unwrap().len());
            let local_path = dent.path().strip_prefix(&base_path).unwrap();
            let url = format!("{}/{}", url_prefix, path_as_url(local_path));
            processor.add(&url, dent.path())?;
        }
    }

    if matches.is_present("validate") {
        println!("Running with sourcemap validation");
        processor.validate_all()?;
    }

    if matches.is_present("rewrite") {
        let mut prefixes: Vec<&str> = match matches.values_of("strip_prefix") {
            Some(paths) => paths.collect(),
            None => vec![],
        };
        if matches.is_present("strip_common_prefix") {
            prefixes.push("~");
        }
        processor.rewrite(&prefixes)?;
    }

    if !matches.is_present("no_sourcemap_reference") {
        processor.add_sourcemap_references()?;
    }

    // make sure the release exists
    let release = api.new_release(&org, &project, &NewRelease {
        version: version.into(),
        ..Default::default()
    })?;
    println!("Uploading sourcemaps for release {}", release.version);
    processor.upload(&api, &org, &project, &release.version)?;

    Ok(())
}

fn execute_files<'a>(matches: &ArgMatches<'a>,
                     config: &Config,
                     org: &str,
                     project: &str)
                     -> Result<()> {
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

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_new(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("finalize") {
        let (org, project) = config.get_org_and_project(matches)?;
        return execute_finalize(sub_matches, config, &org, &project);
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
