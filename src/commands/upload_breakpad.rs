use std::collections::BTreeSet;

use clap::{App, AppSettings, Arg, ArgMatches};
use console::style;
use symbolic_common::ObjectKind;
use uuid::Uuid;

use api::Api;
use config::Config;
use errors::{ErrorKind, Result};
use utils::args::{validate_uuid, ArgExt};
use utils::dif_upload::DifUpload;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload breakpad symbols to a project.")
        .setting(AppSettings::Hidden)
        .org_project_args()
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("A path to search recursively for symbol files.")
            .multiple(true)
            .number_of_values(1)
            .index(1))
        .arg(Arg::with_name("uuids")
            .value_name("UUID")
            .long("uuid")
            .help("Search for specific UUIDs.")
            .validator(validate_uuid)
            .multiple(true)
            .number_of_values(1))
        .arg(Arg::with_name("no_zips")
            .long("no-zips")
            .help("Do not search in ZIP files."))
        .arg(Arg::with_name("require_all")
            .long("require-all")
            .help("Errors if not all UUIDs specified with --uuid could be found."))
        .arg(Arg::with_name("no_reprocessing")
            .long("no-reprocessing")
            .help("Do not trigger reprocessing after uploading."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let api = Api::get_current();
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;

    let uuids = matches
        .values_of("uuids")
        .unwrap_or_default()
        .filter_map(|s| Uuid::parse_str(s).ok());

    // Execute the upload
    let uploaded = DifUpload::new(org.clone(), project.clone())
        .search_paths(matches.values_of("paths").unwrap_or_default())
        .filter_kind(ObjectKind::Breakpad)
        .filter_ids(uuids)
        .allow_zips(!matches.is_present("no_zips"))
        .upload_with(&api)?;

    // Trigger reprocessing only if requested by user
    if matches.is_present("no_reprocessing") {
        println!("{} skipped reprocessing", style(">").dim());
    } else if !api.trigger_reprocessing(&org, &project)? {
        println!("{} Server does not support reprocessing.", style(">").dim());
    }

    // Did we miss explicitly requested symbols?
    if matches.is_present("require_all") {
        let required_uuids: BTreeSet<_> = matches
            .values_of("uuids")
            .unwrap_or_default()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect();

        let found_uuids = uploaded.into_iter().map(|dif| dif.uuid()).collect();
        let missing_uuids: Vec<_> = required_uuids.difference(&found_uuids).collect();

        if !missing_uuids.is_empty() {
            println!("");
            println_stderr!("{}", style("Error: Some symbols could not be found!").red());
            println_stderr!("The following symbols are still missing:");
            for uuid in missing_uuids {
                println!("  {}", uuid);
            }

            return Err(ErrorKind::QuietExit(1).into());
        }
    }

    Ok(())
}
