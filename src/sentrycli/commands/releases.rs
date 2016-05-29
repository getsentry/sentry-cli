use std::path::{Path, PathBuf};
use std::ffi::OsStr;

use clap::{App, AppSettings, Arg, ArgMatches};
use hyper::method::Method;
use hyper::status::StatusCode;
use multipart::client::Multipart;
use serde_json;
use walkdir::WalkDir;

use CliResult;
use commands::Config;
use utils::{make_subcommand, get_org_and_project};

#[derive(Debug, Serialize)]
struct NewRelease {
    version: String,
    #[serde(rename="ref", skip_serializing_if="Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    url: Option<String>
}

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    version: String,
    #[serde(rename="ref")]
    reference: Option<String>,
    url: Option<String>,
    #[serde(rename="dateCreated")]
    date_created: String,
    #[serde(rename="dateReleased")]
    date_released: Option<String>,
    #[serde(rename="newGroups")]
    new_groups: u64,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    sha1: String,
    name: String,
    size: u64,
}

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
        .subcommand(make_subcommand("upload-sourcemaps")
                .about("Uploads sourcemap information for a given release")
                .arg(Arg::with_name("release")
                     .short("r")
                     .long("release")
                     .value_name("VERSION")
                     .required(true)
                     .help("The version identifier of the release to use"))
                .arg(Arg::with_name("paths")
                     .value_name("PATHS")
                     .index(1)
                     .required(true)
                     .multiple(true)
                     .help("The files to upload"))
                .arg(Arg::with_name("url_prefix")
                     .short("u")
                     .long("url-prefix")
                     .required(true)
                     .value_name("PREFIX")
                     .help("The URL prefix to prepend to all filenames"))
                .arg(Arg::with_name("extensions")
                     .long("ext")
                     .short("x")
                     .multiple(true)
                     .help("Add a file extension to the list of files to upload.")))
}

pub fn execute_new<'a>(matches: &ArgMatches<'a>, config: &Config,
                       org: &str, project: &str) -> CliResult<()> {
    let release = NewRelease {
        version: matches.value_of("version").unwrap().to_owned(),
        reference: matches.value_of("ref").map(|x| x.to_owned()),
        url: matches.value_of("url").map(|x| x.to_owned()),
    };
    let mut resp = config.json_api_request(
        Method::Post, &format!("/projects/{}/{}/releases/", org, project),
        &release)?;
    if !resp.status.is_success() {
        fail!(resp);
    } else {
        let info_rv : ReleaseInfo = serde_json::from_reader(&mut resp)?;
        println!("Created release {}.", info_rv.version);
    }
    Ok(())
}

pub fn execute_delete<'a>(matches: &ArgMatches<'a>, config: &Config,
                          org: &str, project: &str) -> CliResult<()> {
    let version = matches.value_of("version").unwrap();
    let resp = config.api_request(
        Method::Delete, &format!("/projects/{}/{}/releases/{}/", org, project, version))?;
    if resp.status == StatusCode::NotFound {
        println!("Did nothing. Release with this version ({}) does not exist.", version);
    } else if !resp.status.is_success() {
        fail!(resp);
    } else {
        println!("Deleted release {}!", version);
    }
    Ok(())
}

pub fn execute_list<'a>(_matches: &ArgMatches<'a>, config: &Config,
                        org: &str, project: &str) -> CliResult<()> {
    let mut resp = config.api_request(
        Method::Get, &format!("/projects/{}/{}/releases/", org, project))?;
    if !resp.status.is_success() {
        fail!(resp);
    } else {
        let infos : Vec<ReleaseInfo> = serde_json::from_reader(&mut resp)?;
        for info in infos {
            println!("[{}] {}: {} ({} new groups)",
                     info.date_released.unwrap_or("              unreleased".into()),
                     info.version,
                     info.reference.unwrap_or("-".into()),
                     info.new_groups);
        }
    }
    Ok(())
}

fn upload_sourcemap(local_path: &Path, url: &str, config: &Config, version: &str,
                    org: &str, project: &str) -> CliResult<Option<Artifact>> {
    let req = config.prepare_api_request(Method::Post,
        &format!("/projects/{}/{}/releases/{}/files/", org, project, version))?;
    let mut mp = Multipart::from_request_sized(req)?;
    mp.write_file("file", &local_path)?;
    mp.write_text("header", "Content-Type:text/plain; encoding=utf-8")?;
    mp.write_text("name", url)?;
    let mut resp = mp.send()?;
    if resp.status == StatusCode::Conflict {
        Ok(None)
    } else if !resp.status.is_success() {
        fail!(resp);
    } else {
        Ok(Some(serde_json::from_reader(&mut resp)?))
    }
}

pub fn execute_upload_sourcemaps<'a>(matches: &ArgMatches<'a>, config: &Config,
                                     org: &str, project: &str) -> CliResult<()> {
    let mut resp = config.api_request(
        Method::Get, &format!("/projects/{}/{}/releases/{}/", org, project,
                              matches.value_of("release").unwrap()))?;
    if !resp.status.is_success() {
        fail!(resp);
    }
    let release : ReleaseInfo = serde_json::from_reader(&mut resp)?;
    let url_prefix = matches.value_of("url_prefix").unwrap().trim_right_matches("/");
    let paths = matches.values_of("paths").unwrap();
    let extensions = match matches.values_of("extensions") {
        Some(matches) => matches.map(|ext| OsStr::new(ext.trim_left_matches("."))).collect(),
        None => vec![OsStr::new("js"), OsStr::new("map")],
    };

    println!("Uploading sourcemaps for release {}", release.version);

    for path in paths {
        let path = PathBuf::from(&path);
        for dent in WalkDir::new(&path) {
            let dent = dent?;
            let extension = dent.path().extension();
            if !extensions.iter().any(|ext| Some(*ext) == extension) {
                continue;
            }
            let local_path = dent.path().strip_prefix(&path).unwrap();
            let url = format!("{}/{}", url_prefix, local_path.display());
            println!("{} -> {}", local_path.display(), url);
            if let Some(artifact) = upload_sourcemap(dent.path(), &url, config,
                                                     &release.version, org, project)? {
                println!("  {} {} bytes", artifact.sha1, artifact.size);
            } else {
                println!("  already present");
            }
        }
    }
    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_new(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_delete(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_list(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("upload-sourcemaps") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_upload_sourcemaps(sub_matches, config, &org, &project);
    }
    unreachable!();
}
