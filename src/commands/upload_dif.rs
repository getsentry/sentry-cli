//! Implements a command for uploading dSYM files.
use std::collections::BTreeSet;
use std::env;
use std::str::{self, FromStr};

use clap::{App, ArgMatches};
use console::style;
use dirs;
use failure::{err_msg, Error};
use indicatif::{ProgressBar, ProgressStyle};
use symbolic::common::types::{ObjectClass, ObjectKind};
use symbolic::debuginfo::DebugId;

use api::Api;
use config::Config;
use utils::args::{validate_id, validate_org, validate_project};
use utils::dif_upload::DifUpload;
use utils::system::QuietExit;
use utils::xcode::{InfoPlist, MayDetach};

static DERIVED_DATA: &'static str = "Library/Developer/Xcode/DerivedData";

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Upload debugging information files.")
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} "The project slug.")
        (@arg paths: [PATH]... "A path to search recursively for symbol files.")
        (@arg types: -t --type [TYPE]... possible_values(&["dsym", "elf", "breakpad"])
            "Only consider debug information files of the given \
             type.  By default, all types are considered.")
        (@arg no_executables: --("no-bin") conflicts_with[no_debug_only]
            "Exclude executables and libraries and look for debug symbols only.")
        (@arg no_debug_only: --("no-debug") conflicts_with[no_executables]
            "Exclude files containing only stripped debugging info.")
        (@arg ids: --id [ID]... {validate_id} "Search for specific debug identifiers.")
        (@arg require_all: --("require-all")
            "Errors if not all identifiers specified with --id could be found.")
        (@arg symbol_maps: --("symbol-maps") [PATH]
            "Optional path to BCSymbolMap files which are used to \
             resolve hidden symbols in dSYM files downloaded from \
             iTunes Connect.  This requires the dsymutil tool to be \
             available.")
        (@arg derived_data: --("derived-data") "Search for debug symbols in Xcode's derived data.")
        (@arg no_zips: --("no-zips") "Do not search in ZIP files.")
        (@arg info_plist: --("info-plist") [PATH]
            "Optional path to the Info.plist.{n}We will try to find this \
             automatically if run from Xcode.  Providing this information \
             will associate the debug symbols with a specific ITC application \
             and build in Sentry.  Note that if you provide the plist \
             explicitly it must already be processed.")
        (@arg no_reprocessing: --("no-reprocessing") "Do not trigger reprocessing after uploading.")
        (@arg force_foreground: --("force-foreground")
            "Wait for the process to finish.{n}\
             By default, the upload process will detach and continue in the \
             background when triggered from Xcode.  When an error happens, \
             a dialog is shown.  If this parameter is passed Xcode will wait \
             for the process to finish before the build finishes and output \
             will be shown in the Xcode build output.")
    )
}

fn execute_internal(matches: &ArgMatches, legacy: bool) -> Result<(), Error> {
    let api = Api::get_current();
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;

    let ids = matches
        .values_of("ids")
        .unwrap_or_default()
        .filter_map(|s| DebugId::from_str(s).ok());

    // Build generic upload parameters
    let mut upload = DifUpload::new(org.clone(), project.clone());
    upload
        .search_paths(matches.values_of("paths").unwrap_or_default())
        .allow_zips(!matches.is_present("no_zips"))
        .filter_ids(ids);

    if legacy {
        // Configure `upload-dsym` behavior (only dSYM files)
        upload
            .filter_kind(ObjectKind::MachO)
            .filter_class(ObjectClass::Debug);

        if !matches.is_present("paths") {
            if let Some(dsym_path) = env::var_os("DWARF_DSYM_FOLDER_PATH") {
                upload.search_path(dsym_path);
            }
        }
    } else {
        // Restrict symbol types, if specified by the user
        for ty in matches.values_of("types").unwrap_or_default() {
            upload.filter_kind(match ty {
                "dsym" => ObjectKind::MachO,
                "elf" => ObjectKind::Elf,
                "breakpad" => ObjectKind::Breakpad,
                other => bail!("Unsupported type: {}", other),
            });
        }

        // Allow executables and dynamic/shared libraries, but not object fiels.
        // They may optionally contain debugging information (such as DWARF) or
        // stackwalking info (for instance `eh_frame`).
        if !matches.is_present("no_executables") {
            upload
                .filter_class(ObjectClass::Executable)
                .filter_class(ObjectClass::Library);
        }

        // Allow stripped debug symbols. These are dSYMs, ELF binaries generated
        // with `objcopy --only-keep-debug` or Breakpad symbols.
        if !matches.is_present("no_debug_only") {
            upload.filter_class(ObjectClass::Debug);
        }
    }

    // Configure BCSymbolMap resolution, if possible
    if let Some(symbol_map) = matches.value_of("symbol_maps") {
        upload
            .symbol_map(symbol_map)
            .map_err(|_| err_msg("--symbol-maps requires Apple dsymutil to be available."))?;
    }

    // Add a path to XCode's DerivedData, if configured
    if matches.is_present("derived_data") {
        let derived_data = dirs::home_dir().map(|x| x.join(DERIVED_DATA));
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
        let uploaded = upload.upload()?;

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
                if association.associated_dsyms.is_empty() {
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
            let required_ids: BTreeSet<_> = matches
                .values_of("ids")
                .unwrap_or_default()
                .filter_map(|s| DebugId::from_str(s).ok())
                .collect();

            let found_ids = uploaded.into_iter().map(|dif| dif.id()).collect();
            let missing_ids: Vec<_> = required_ids.difference(&found_ids).collect();

            if !missing_ids.is_empty() {
                println!();
                println_stderr!("{}", style("Error: Some symbols could not be found!").red());
                println_stderr!("The following symbols are still missing:");
                for id in missing_ids {
                    println!("  {}", id);
                }

                return Err(QuietExit(1).into());
            }
        }

        Ok(())
    })
}

pub fn execute(matches: &ArgMatches) -> Result<(), Error> {
    execute_internal(matches, false)
}

pub fn execute_legacy(matches: &ArgMatches) -> Result<(), Error> {
    execute_internal(matches, true)
}
