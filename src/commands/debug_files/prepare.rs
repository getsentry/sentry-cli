use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use log::info;
use serde::Serialize;
use symbolic::debuginfo::FileFormat;
use uuid::Uuid;

use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::ArgExt as _;
use crate::utils::dif::ObjectDifFeatures;
use crate::utils::dif_upload::{DifFormat, DifUpload};
use crate::utils::logging::is_quiet_mode;
use crate::utils::system::QuietExit;
use crate::utils::wasm::{
    is_debug_companion_path, is_wasm_path, prepare_wasm_file, PrepareAction, PrepareOptions,
    PrepareResult,
};

pub fn make_command(command: Command) -> Command {
    command
        .about("Prepare WebAssembly files for Sentry: split DWARF, then upload companions.")
        .long_about(
            "Prepare WebAssembly debug files for Sentry.{n}{n}\
             Scans PATH for .wasm modules, classifies debug quality, and for \
             modules with DWARF runs the same split as wasm-split: inject a \
             build_id if missing, write a *.debug.wasm companion that keeps \
             the Code section, and strip DWARF from the deployable .wasm.{n}{n}\
             Companions are uploaded with debug-files upload --type wasm \
             unless --no-upload or --dry-run is set.{n}{n}\
             Compiler-agnostic: operates on module sections only. Build with \
             DWARF (Emscripten -g, wasm-pack dwarf-debug-info) and optionally \
             --build-id at link. Name/symtab-only modules are skipped with a \
             warning (no line-level symbolication).",
        )
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("A .wasm file or a directory to search recursively.")
                .num_args(1..)
                .required(true)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("ignore")
                .long("ignore")
                .short('i')
                .value_name("IGNORE")
                .action(ArgAction::Append)
                .help("Ignores all files and folders matching the given glob."),
        )
        .arg(
            Arg::new("ignore_file")
                .long("ignore-file")
                .short('I')
                .value_name("IGNORE_FILE")
                .help(
                    "Ignore all files and folders specified in the given \
                     ignore file, e.g. .gitignore.",
                ),
        )
        .arg(
            Arg::new("out_dir")
                .long("out-dir")
                .value_name("DIR")
                .help(
                    "Write *.debug.wasm companions into this directory. \
                     The stripped .wasm is still written next to the input \
                     (in-place) so the original path stays the deploy artifact.",
                ),
        )
        .arg(
            Arg::new("no_upload")
                .long("no-upload")
                .action(ArgAction::SetTrue)
                .help("Split only; do not upload companions."),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Inspect and classify modules without writing or uploading."),
        )
        .arg(
            Arg::new("require_dwarf")
                .long("require-dwarf")
                .action(ArgAction::SetTrue)
                .help("Exit with an error if any scanned .wasm lacks DWARF (or a matching companion)."),
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
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Format outputs as JSON."),
        )
        .arg(
            Arg::new("build_id")
                .long("build-id")
                .value_name("UUID")
                .value_parser(Uuid::parse_str)
                .help(
                    "Explicit build_id to inject when a module has none. \
                     Defaults to a random UUID.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {

    // Read CLI arguments 
    #[expect(clippy::unwrap_used, reason = "required clap argument")]
    let paths: Vec<PathBuf> = matches
        .get_many::<String>("paths")
        .unwrap()
        .map(PathBuf::from)
        .collect();

    let dry_run = matches.get_flag("dry_run");
    let no_upload = matches.get_flag("no_upload") || dry_run;
    let require_dwarf = matches.get_flag("require_dwarf");
    let json = matches.get_flag("json");
    let quiet = is_quiet_mode();
    let out_dir = matches.get_one::<String>("out_dir").map(Path::new);
    let build_id = matches.get_one::<Uuid>("build_id").copied();

    let ignore_file = matches
        .get_one::<String>("ignore_file")
        .map(String::as_str)
        .unwrap_or_default();
    let ignores: Vec<_> = matches
        .get_many::<String>("ignore")
        .map(|ignores| ignores.map(|i| format!("!{i}")).collect())
        .unwrap_or_default();

    // Collect WASM files from the given paths
    let mut wasm_files = Vec::new();
    for path in &paths {
        if !path.exists() {
            bail!("Given path does not exist: {}", path.display());
        }
        if !json && !quiet {
            println!("{} Searching {}", style(">").dim(), path.display());
        }
        wasm_files.extend(collect_wasm_files(path, ignore_file, &ignores)?);
    }
    wasm_files.sort();
    wasm_files.dedup();

    if !json && !quiet {
        println!(
            "{} Found {} {}",
            style(">").dim(),
            style(wasm_files.len()).yellow(),
            match wasm_files.len() {
                1 => "wasm file",
                _ => "wasm files",
            }
        );
    }

    // Prepare WASM files
    let options = PrepareOptions {
        dry_run,
        out_dir,
        build_id,
    };

    let mut results = Vec::new();
    for wasm_path in &wasm_files {
        info!("preparing {}", wasm_path.display());
        let result = prepare_wasm_file(wasm_path, options)?;
        if !json && !quiet {
            print_result(&result);
        }
        results.push(result);
    }

    // JSON + strict checks
    let dwarf_missing = results
        .iter()
        .any(|result| result.action == PrepareAction::Skipped && !result.quality.has_dwarf());

    if json {
        serde_json::to_writer_pretty(&mut io::stdout(), &PrepareReport { files: &results })?;
        println!();
    }

    if require_dwarf && dwarf_missing {
        if !json && !quiet {
            eprintln!(
                "{}",
                style("Error: some .wasm files lack DWARF (--require-dwarf)").red()
            );
        }
        return Err(QuietExit(1).into());
    }

    // Upload companions or stop 
    if no_upload {
        return Ok(());
    }

    let companions: Vec<PathBuf> = results
        .iter()
        .filter(|result| {
            matches!(
                result.action,
                PrepareAction::Split | PrepareAction::AlreadyPrepared
            )
        })
        .filter_map(|result| result.companion.clone())
        .collect();

    if companions.is_empty() {
        if !json && !quiet {
            println!("{} No companions to upload", style(">").dim());
        }
        return Ok(());
    }

    upload_companions(matches, &companions)
}

#[derive(Serialize)]
struct PrepareReport<'a> {
    files: &'a [PrepareResult],
}

fn print_result(result: &PrepareResult) {
    let header = match result.action {
        PrepareAction::Split => format!("{} Split {}", style(">").dim(), result.path.display()),
        PrepareAction::WouldSplit => {
            format!("{} Would split {}", style(">").dim(), result.path.display())
        }
        PrepareAction::AlreadyPrepared => format!(
            "{} Already prepared {}",
            style(">").dim(),
            result.path.display()
        ),
        PrepareAction::Skipped => {
            format!("{} Skipping {}", style(">").dim(), result.path.display())
        }
    };
    println!("{header}");
    println!("    Debug quality: {}", result.quality.as_str());
    if let Some(build_id) = &result.build_id {
        println!("    Build ID: {build_id}");
    }
    if let Some(companion) = &result.companion {
        println!("    Companion: {}", companion.display());
    }
    if let Some(warning) = &result.warning {
        println!("    {}: {warning}", style("Warning").yellow());
    }
}

fn collect_wasm_files(path: &Path, ignore_file: &str, ignores: &[String]) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        if is_wasm_path(path) {
            return Ok(vec![path.to_path_buf()]);
        }
        bail!(
            "Expected a .wasm file or a directory, but got {}",
            path.display()
        );
    }

    let mut builder = WalkBuilder::new(path);
    builder.follow_links(true);
    builder.sort_by_file_name(|a, b| a.cmp(b));
    builder.git_exclude(false).git_ignore(false).ignore(false);

    let mut types_builder = TypesBuilder::new();
    types_builder.add("wasm", "*.wasm")?;
    builder.types(types_builder.select("wasm").build()?);

    if !ignore_file.is_empty() {
        builder.add_ignore(ignore_file);
    }

    if !ignores.is_empty() {
        let mut override_builder = OverrideBuilder::new(path);
        for ignore in ignores {
            override_builder.add(ignore)?;
        }
        builder.overrides(override_builder.build()?);
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let file = entry?;
        if file.file_type().is_some_and(|t| t.is_dir()) {
            continue;
        }
        let file_path = file.path();
        if is_debug_companion_path(file_path) {
            continue;
        }
        if is_wasm_path(file_path) {
            files.push(file_path.to_path_buf());
        }
    }
    Ok(files)
}

fn upload_companions(matches: &ArgMatches, companions: &[PathBuf]) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;

    let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
    let wait = matches.get_flag("wait") || wait_for_secs.is_some();
    let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, Duration::from_secs);

    let mut upload = DifUpload::new(&org, &project);
    upload
        .wait(wait)
        .max_wait(max_wait)
        .search_paths(companions.iter().cloned())
        .allow_zips(false)
        .filter_format(DifFormat::Object(FileFormat::Wasm))
        .filter_features(ObjectDifFeatures {
            debug: true,
            symtab: true,
            unwind: true,
            sources: true,
        })
        .include_sources(matches.get_flag("include_sources"));

    let (_uploaded, has_processing_errors) = upload.upload()?;
    if has_processing_errors {
        eprintln!();
        eprintln!("{}", style("Error: some symbols did not process correctly"));
        return Err(QuietExit(1).into());
    }
    Ok(())
}
