//! Implements a command for uploading dSYM files.
use std::collections::BTreeSet;
use std::env;
use std::str;

use clap::{App, Arg, ArgMatches};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use symbolic_common::{ObjectClass, ObjectKind};
use uuid::Uuid;

use api::Api;
use config::Config;
use errors::{ErrorKind, Result};
use utils::args::{validate_uuid, ArgExt};
use utils::dif_upload::DifUpload;
use utils::xcode::{InfoPlist, MayDetach};

static DERIVED_DATA: &'static str = "Library/Developer/Xcode/DerivedData";

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload Mac debug symbols to a project.")
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
        .arg(Arg::with_name("require_all")
             .long("require-all")
             .help("Errors if not all UUIDs specified with --uuid could be found."))
        .arg(Arg::with_name("symbol_maps")
             .long("symbol-maps")
             .value_name("PATH")
             .help("Optional path to BCSymbolMap files which are used to \
                    resolve hidden symbols in the actual dSYM files.  This \
                    requires the dsymutil tool to be available."))
        .arg(Arg::with_name("derived_data")
             .long("derived-data")
             .help("Search for debug symbols in derived data."))
        .arg(Arg::with_name("no_zips")
             .long("no-zips")
             .help("Do not search in ZIP files."))
        .arg(Arg::with_name("info_plist")
             .long("info-plist")
             .value_name("PATH")
             .help("Optional path to the Info.plist.{n}We will try to find this \
                    automatically if run from Xcode.  Providing this information \
                    will associate the debug symbols with a specific ITC application \
                    and build in Sentry.  Note that if you provide the plist \
                    explicitly it must already be processed."))
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Do not trigger reprocessing after uploading."))
        .arg(Arg::with_name("force_foreground")
             .long("force-foreground")
             .help("Wait for the process to finish.{n}\
                    By default, the upload process will detach and continue in the \
                    background when triggered from Xcode.  When an error happens, \
                    a dialog is shown.  If this parameter is passed Xcode will wait \
                    for the process to finish before the build finishes and output \
                    will be shown in the Xcode build output."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let api = Api::get_current();
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;

    let uuids = matches
        .values_of("uuids")
        .unwrap_or_default()
        .filter_map(|s| Uuid::parse_str(s).ok());

    // Build generic upload parameters
    let mut upload = DifUpload::new(org.clone(), project.clone());
    upload
        .search_paths(matches.values_of("paths").unwrap_or_default())
        .filter_kind(ObjectKind::MachO)
        .filter_class(ObjectClass::Debug)
        .filter_ids(uuids)
        .allow_zips(!matches.is_present("no_zips"));

    // Configure BCSymbolMap resolution, if possible
    if let Some(symbol_map) = matches.value_of("symbol_maps") {
        upload
            .symbol_map(symbol_map)
            .map_err(|_| "--symbol-maps requires Apple dsymutil to be available.")?;
    }

    // Add a path to XCode's DerivedData, if configured
    if matches.is_present("derived_data") {
        let derived_data = env::home_dir().map(|x| x.join(DERIVED_DATA));
        if let Some(path) = derived_data {
            if path.is_dir() {
                upload.search_path(path);
            }
        }
    }

    // Try to resolve the Info.plist either by path or from Xcode
    let info_plist = match matches.value_of("info_plist") {
        Some(path) => Some(InfoPlist::from_path(path)?),
        None => InfoPlist::discover_from_env()?,
    };

    MayDetach::wrap("Debug symbol upload", |handle| {
        // Optionally detach if run from Xcode
        if !matches.is_present("force_foreground") {
            handle.may_detach()?;
        }

        // Execute the upload
        let uploaded = upload.upload_with(&api)?;

        // Associate the dSYMs with the Info.plist data, if available
        if let Some(ref info_plist) = info_plist {
            let progress_style = ProgressStyle::default_spinner()
                .template("{spinner} Associating dSYMs with {msg}...");

            let progress = ProgressBar::new_spinner();
            progress.enable_steady_tick(100);
            progress.set_style(progress_style);
            progress.set_message(&info_plist.to_string());

            let checksums = uploaded.iter().map(|dif| dif.checksum.clone()).collect();
            let response = api.associate_apple_dsyms(&org, &project, info_plist, checksums)?;
            progress.finish_and_clear();

            if let Some(association) = response {
                if association.associated_dsyms.len() == 0 {
                    println!("{} No new debug symbols to associate.", style(">").dim());
                } else {
                    println!(
                        "{} Associated {} debug symbols with the build.",
                        style(">").dim(),
                        style(association.associated_dsyms.len()).yellow()
                    );
                }
            } else {
                info!("Server does not support dSYM associations. Ignoring.");
            }
        }

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
    })
}
