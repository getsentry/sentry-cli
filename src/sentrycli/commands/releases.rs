use clap::{App, Arg, ArgMatches, AppSettings};
use hyper::method::Method;
use hyper::status::StatusCode;
use serde_json;

use CliResult;
use commands::Config;
use utils::{make_subcommand, get_org_and_project};

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseInfo {
    version: String,
    #[serde(rename="ref", skip_serializing_if="Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    url: Option<String>,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("manage releases on Sentry")
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
        .setting(AppSettings::ArgRequiredElseHelp)
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
}

pub fn execute_new<'a>(matches: &ArgMatches<'a>, config: &Config,
                       org: &str, project: &str) -> CliResult<()> {
    let info = ReleaseInfo {
        version: matches.value_of("version").unwrap().to_owned(),
        reference: matches.value_of("ref").map(|x| x.to_owned()),
        url: matches.value_of("url").map(|x| x.to_owned()),
    };
    let mut resp = config.json_api_request(
        Method::Post, &format!("/projects/{}/{}/releases/", org, project),
        &info)?;
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

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_new(sub_matches, config, &org, &project);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        let (org, project) = get_org_and_project(matches)?;
        return execute_delete(sub_matches, config, &org, &project);
    }
    Ok(())
}
