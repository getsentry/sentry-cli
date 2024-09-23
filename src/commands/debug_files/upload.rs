use std::collections::BTreeSet;
use std::str::{self, FromStr};
use std::time::Duration;

use anyhow::{bail, format_err, Result};
use clap::{builder::PossibleValuesParser, Arg, ArgAction, ArgMatches, Command};
use console::style;
use itertools::Itertools;
use log::info;
use symbolic::common::DebugId;
use symbolic::debuginfo::FileFormat;

use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::ArgExt;
use crate::utils::dif::{DifType, ObjectDifFeatures};
use crate::utils::dif_upload::{DifFormat, DifUpload};
use crate::utils::system::QuietExit;
use crate::utils::xcode::{InfoPlist, MayDetach};

static DERIVED_DATA_FOLDER: &str = "Library/Developer/Xcode/DerivedData";

pub fn make_command(command: Command) -> Command {
    let types = DifType::all_names()
        .iter()
        .filter(|&name| name != &"proguard")
        .chain(&["bcsymbolmap"])
        .sorted_by(|a, b| Ord::cmp(&a, &b))
        .collect::<Vec<_>>();

    command
        .about("Upload debugging information files.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("A path to search recursively for symbol files.")
                .num_args(1..)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("types")
                .long("type")
                .short('t')
                .value_name("TYPE")
                .action(ArgAction::Append)
                .value_parser(PossibleValuesParser::new(types))
                .help(
                    "Only consider debug information files of the given \
                    type.  By default, all types are considered.",
                ),
        )
        .arg(
            Arg::new("no_unwind")
                .long("no-unwind")
                .alias("no-bin")
                .action(ArgAction::SetTrue)
                .help(
                    "Do not scan for stack unwinding information. Specify \
                    this flag for builds with disabled FPO, or when \
                    stackwalking occurs on the device. This usually \
                    excludes executables and dynamic libraries. They might \
                    still be uploaded, if they contain additional \
                    processable information (see other flags).",
                ),
        )
        .arg(
            Arg::new("no_debug")
                .long("no-debug")
                .action(ArgAction::SetTrue)
                .help(
                    "Do not scan for debugging information. This will \
                    usually exclude debug companion files. They might \
                    still be uploaded, if they contain additional \
                    processable information (see other flags).",
                )
                .conflicts_with("no_unwind"),
        )
        .arg(
            Arg::new("no_sources")
                .long("no-sources")
                .action(ArgAction::SetTrue)
                .help(
                    "Do not scan for source information. This will \
                    usually exclude source bundle files. They might \
                    still be uploaded, if they contain additional \
                    processable information (see other flags).",
                ),
        )
        .arg(
            Arg::new("ids")
                .value_name("ID")
                .long("id")
                .help("Search for specific debug identifiers.")
                .value_parser(DebugId::from_str)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("require_all")
                .long("require-all")
                .action(ArgAction::SetTrue)
                .help("Errors if not all identifiers specified with --id could be found."),
        )
        .arg(
            Arg::new("symbol_maps")
                .long("symbol-maps")
                .value_name("PATH")
                .help(
                    "Optional path to BCSymbolMap files which are used to \
                    resolve hidden symbols in dSYM files downloaded from \
                    iTunes Connect.  This requires the dsymutil tool to be \
                    available.  This should not be used when using the App \
                    Store Connect integration, the .bcsymbolmap files needed \
                    for the integration are uploaded without this option if \
                    they are found in the PATH searched for symbol files.",
                ),
        )
        .arg(
            Arg::new("derived_data")
                .long("derived-data")
                .action(ArgAction::SetTrue)
                .help("Search for debug symbols in Xcode's derived data."),
        )
        .arg(
            Arg::new("no_zips")
                .long("no-zips")
                .action(ArgAction::SetTrue)
                .help("Do not search in ZIP files."),
        )
        .arg(
            Arg::new("info_plist")
                .long("info-plist")
                .value_name("PATH")
                .help(
                    "Optional path to the Info.plist.{n}We will try to find this \
                    automatically if run from Xcode.  Providing this information \
                    will associate the debug symbols with a specific ITC application \
                    and build in Sentry.  Note that if you provide the plist \
                    explicitly it must already be processed.",
                ),
        )
        .arg(
            Arg::new("no_upload")
                .long("no-upload")
                .action(ArgAction::SetTrue)
                .help(
                    "Disable the actual upload.{n}This runs all steps for the \
                    processing but does not trigger the upload.  This is useful if you \
                    just want to verify the setup or skip the upload in tests.",
                ),
        )
        .arg(
            Arg::new("force_foreground")
                .long("force-foreground")
                .action(ArgAction::SetTrue)
                .help(
                    "Wait for the process to finish.{n}\
                    By default, the upload process will detach and continue in the \
                    background when triggered from Xcode.  When an error happens, \
                    a dialog is shown.  If this parameter is passed Xcode will wait \
                    for the process to finish before the build finishes and output \
                    will be shown in the Xcode build output.",
                ),
        )
        .arg(
            Arg::new("include_sources")
                .long("include-sources")
                .action(ArgAction::SetTrue)
                .help(
                    "Include sources from the local file system and upload them as source bundles.",
                ),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .conflicts_with("wait_for")
                .help(
                    "Wait for the server to fully process uploaded files. Errors \
                    can only be displayed if --wait or --wait-for is specified, but this will \
                    significantly slow down the upload process.",
                ),
        )
        .arg(
            Arg::new("wait_for")
                .long("wait-for")
                .value_name("SECS")
                .value_parser(clap::value_parser!(u64))
                .conflicts_with("wait")
                .help(
                    "Wait for the server to fully process uploaded files, \
                    but at most for the given number of seconds. Errors \
                    can only be displayed if --wait or --wait-for is specified, but this will \
                    significantly slow down the upload process.",
                ),
        )
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(
            Arg::new("upload_symbol_maps")
                .long("upload-symbol-maps")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new("il2cpp_mapping")
                .long("il2cpp-mapping")
                .action(ArgAction::SetTrue)
                .help("Compute il2cpp line mappings and upload them along with sources."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;

    let ids = matches
        .get_many::<DebugId>("ids")
        .unwrap_or_default()
        .copied();

    info!(
        "Issuing a command for Organization: {} Project: {}",
        org, project
    );

    let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
    let wait = matches.get_flag("wait") || wait_for_secs.is_some();
    let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, Duration::from_secs);

    // Build generic upload parameters
    let mut upload = DifUpload::new(org.clone(), project.clone());
    upload
        .wait(wait)
        .max_wait(max_wait)
        .search_paths(matches.get_many::<String>("paths").unwrap_or_default())
        .allow_zips(!matches.get_flag("no_zips"))
        .filter_ids(ids);

    // Restrict symbol types, if specified by the user
    for ty in matches
        .get_many::<String>("types")
        .unwrap_or_default()
        .map(String::as_str)
    {
        match ty {
            "dsym" => upload.filter_format(DifFormat::Object(FileFormat::MachO)),
            "elf" => upload.filter_format(DifFormat::Object(FileFormat::Elf)),
            "breakpad" => upload.filter_format(DifFormat::Object(FileFormat::Breakpad)),
            "pdb" => upload.filter_format(DifFormat::Object(FileFormat::Pdb)),
            "pe" => upload.filter_format(DifFormat::Object(FileFormat::Pe)),
            "sourcebundle" => upload.filter_format(DifFormat::Object(FileFormat::SourceBundle)),
            "portablepdb" => upload.filter_format(DifFormat::Object(FileFormat::PortablePdb)),
            "jvm" => upload.filter_format(DifFormat::Object(FileFormat::SourceBundle)),
            "wasm" => upload.filter_format(DifFormat::Object(FileFormat::Wasm)),
            "bcsymbolmap" => {
                upload.filter_format(DifFormat::BcSymbolMap);
                upload.filter_format(DifFormat::PList)
            }
            other => bail!("Unsupported type: {}", other),
        };
    }

    upload.filter_features(ObjectDifFeatures {
        // Allow stripped debug symbols. These are dSYMs, ELF binaries generated
        // with `objcopy --only-keep-debug` or Breakpad symbols. As a fallback,
        // we also upload all files with a public symbol table.
        debug: !matches.get_flag("no_debug"),
        symtab: !matches.get_flag("no_debug"),
        // Allow executables and dynamic/shared libraries, but not object files.
        // They are guaranteed to contain unwind info, for instance `eh_frame`,
        // and may optionally contain debugging information such as DWARF.
        unwind: !matches.get_flag("no_unwind"),
        sources: !matches.get_flag("no_sources"),
    });

    upload.include_sources(matches.get_flag("include_sources"));
    upload.il2cpp_mapping(matches.get_flag("il2cpp_mapping"));

    // Configure BCSymbolMap resolution, if possible
    if let Some(symbol_map) = matches.get_one::<String>("symbol_maps") {
        upload
            .symbol_map(symbol_map)
            .map_err(|_| format_err!("--symbol-maps requires Apple dsymutil to be available."))?;
    }

    // Add a path to XCode's DerivedData, if configured
    if matches.get_flag("derived_data") {
        let derived_data = dirs::home_dir().map(|x| x.join(DERIVED_DATA_FOLDER));
        if let Some(path) = derived_data {
            if path.is_dir() {
                upload.search_path(path);
            }
        }
    }

    // Try to resolve the Info.plist either by path or from Xcode
    // TODO: maybe remove this completely?
    let _info_plist = match matches.get_one::<String>("info_plist") {
        Some(path) => Some(InfoPlist::from_path(path)?),
        None => InfoPlist::discover_from_env()?,
    };

    if matches.get_flag("no_upload") {
        println!("{} skipping upload.", style(">").dim());
        return Ok(());
    }

    MayDetach::wrap("Debug symbol upload", |handle| {
        // Optionally detach if run from Xcode
        if !matches.get_flag("force_foreground") {
            handle.may_detach()?;
        }

        // Execute the upload
        let (uploaded, has_processing_errors) = upload.upload()?;

        // Did we miss explicitly requested symbols?
        if matches.get_flag("require_all") {
            let required_ids: BTreeSet<DebugId> = matches
                .get_many::<DebugId>("ids")
                .unwrap_or_default()
                .cloned()
                .collect();

            let found_ids = uploaded.into_iter().map(|dif| dif.id()).collect();
            let missing_ids: Vec<_> = required_ids.difference(&found_ids).collect();

            if !missing_ids.is_empty() {
                eprintln!();
                eprintln!("{}", style("Error: Some symbols could not be found!").red());
                eprintln!("The following symbols are still missing:");
                for id in missing_ids {
                    println!("  {id}");
                }

                return Err(QuietExit(1).into());
            }
        }

        // report a non 0 status code if the server encountered issues.
        if has_processing_errors {
            eprintln!();
            eprintln!("{}", style("Error: some symbols did not process correctly"));
            return Err(QuietExit(1).into());
        }

        Ok(())
    })
}
