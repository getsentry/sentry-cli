use std::fs::{self, File};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Arg, ArgMatches, Command};
use glob::glob;
use log::{debug, warn};
use serde_json::Value;
use symbolic::debuginfo::js;
use uuid::Uuid;

const CODE_SNIPPET_TEMPLATE: &str = r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="__SENTRY_DEBUG_ID__")}catch(e){}}()"#;
const DEBUGID_PLACEHOLDER: &str = "__SENTRY_DEBUG_ID__";
const SOURCEMAP_DEBUGID_KEY: &str = "debug_id";
const DEBUGID_COMMENT_PREFIX: &str = "//# debugId";

pub fn make_command(command: Command) -> Command {
    command
        .about("Fixes up JavaScript source files and sourcemaps with debug ids.")
        .long_about(
            "Fixes up JavaScript source files and sourcemaps with debug ids.{n}{n}\
            For every JS source file that references a sourcemap, a debug id is generated and \
            inserted into both files. If the referenced sourcemap already contains a debug id, \
            that id is used instead.",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path or glob to the javascript files."),
        )
        .hide(true)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = matches.get_one::<String>("path").unwrap();

    let collected_paths: Vec<PathBuf> = glob(path)
        .unwrap()
        .flatten()
        .filter(|path| path.extension().map_or(false, |ext| ext == "js"))
        .collect();

    if collected_paths.is_empty() {
        warn!("Did not match any JavaScript files for pattern: {}", path);
        return Ok(());
    }

    fixup_files(&collected_paths)
}

fn fixup_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let js_path = path.as_path();

        debug!("Processing js file {}", js_path.display());

        let file =
            fs::read_to_string(js_path).context(format!("Failed to open {}", js_path.display()))?;

        if js::discover_debug_id(&file).is_some() {
            debug!("File {} was previously processed", js_path.display());
            continue;
        }

        let Some(sourcemap_url) = js::discover_sourcemaps_location(&file) else {
            debug!("File {} does not contain a sourcemap url", js_path.display());
            continue;
        };

        let sourcemap_path = js_path.with_file_name(sourcemap_url);

        if !sourcemap_path.exists() {
            warn!("Sourcemap file {} not found", sourcemap_path.display());
            continue;
        }

        let debug_id = fixup_sourcemap(&sourcemap_path)
            .context(format!("Failed to process {}", sourcemap_path.display()))?;

        fixup_js_file(js_path, debug_id)
            .context(format!("Failed to process {}", js_path.display()))?;
    }

    Ok(())
}

/// Appends the following text to a file:
/// ```
///
/// <CODE_SNIPPET>[<debug_id>]
/// //# sentryDebugId=<debug_id>
///```
/// where `<CODE_SNIPPET>[<debug_id>]`
/// is `CODE_SNIPPET_TEMPLATE` with `debug_id` substituted for the `__SENTRY_DEBUG_ID__`
/// placeholder.
fn fixup_js_file(js_path: &Path, debug_id: Uuid) -> Result<()> {
    let mut js_file = File::options().append(true).open(js_path)?;
    let to_inject =
        CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.hyphenated().to_string());
    writeln!(js_file)?;
    writeln!(js_file, "{to_inject}")?;
    write!(js_file, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    Ok(())
}

/// Fixes up a sourcemap file with a debug id.
///
/// If the file already contains a debug id under the `debugID` key, it is left unmodified.
/// Otherwise, a fresh debug id is inserted under that key.
///
/// In either case, the value of the `debugID` key is returned.
fn fixup_sourcemap(sourcemap_path: &Path) -> Result<Uuid> {
    let mut sourcemap_file = File::options()
        .read(true)
        .write(true)
        .open(sourcemap_path)?;
    let mut sourcemap: Value = serde_json::from_reader(&sourcemap_file)?;

    sourcemap_file.rewind()?;

    let Some(map) = sourcemap.as_object_mut() else {
        bail!("Invalid sourcemap");
    };

    match map.get(SOURCEMAP_DEBUGID_KEY) {
        Some(id) => {
            let debug_id = serde_json::from_value(id.clone())?;
            debug!("Sourcemap already has a debug id");
            Ok(debug_id)
        }

        None => {
            let debug_id = Uuid::new_v4();
            let id = serde_json::to_value(debug_id)?;
            map.insert(SOURCEMAP_DEBUGID_KEY.to_string(), id);

            serde_json::to_writer(&mut sourcemap_file, &sourcemap)?;
            Ok(debug_id)
        }
    }
}
