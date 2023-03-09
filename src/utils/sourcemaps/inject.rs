use std::fs::File;
use std::io::{BufRead, BufReader, Seek, Write};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use anyhow::{bail, Context, Result};
use log::debug;
use sentry::types::DebugId;
use serde_json::Value;
use symbolic::debuginfo::js;
use uuid::Uuid;

const CODE_SNIPPET_TEMPLATE: &str = r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="__SENTRY_DEBUG_ID__")}catch(e){}}()"#;
const DEBUGID_PLACEHOLDER: &str = "__SENTRY_DEBUG_ID__";
const SOURCEMAP_DEBUGID_KEY: &str = "debug_id";
const DEBUGID_COMMENT_PREFIX: &str = "//# debugId";

#[derive(Debug, Clone, Default)]
pub struct InjectReport {
    injected: Vec<(PathBuf, DebugId)>,
    previously_injected: Vec<(PathBuf, DebugId)>,
    skipped: Vec<PathBuf>,
    missing_sourcemaps: Vec<PathBuf>,
    sourcemaps: Vec<(PathBuf, DebugId)>,
    skipped_sourcemaps: Vec<(PathBuf, DebugId)>,
}

impl fmt::Display for InjectReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.injected.is_empty() {
            writeln!(
                f,
                "Modified: The following source files have been modified to have debug ids"
            )?;
            for (path, debug_id) in &self.injected {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.sourcemaps.is_empty() {
            writeln!(
                f,
                "\nModified: The following sourcemap files have been modified to have debug ids"
            )?;
            for (path, debug_id) in &self.sourcemaps {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.previously_injected.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files already have debug ids"
            )?;
            for (path, debug_id) in &self.previously_injected {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.skipped_sourcemaps.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following sourcemap files already have debug ids"
            )?;
            for (path, debug_id) in &self.skipped_sourcemaps {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.skipped.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files don't have sourcemap references "
            )?;
            for path in &self.skipped {
                writeln!(f, "  - {}", path.display())?;
            }
        }

        if !self.missing_sourcemaps.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files refer to sourcemaps that couldn't be found"
            )?;
            for path in &self.missing_sourcemaps {
                writeln!(f, "  - {}", path.display())?;
            }
        }

        Ok(())
    }
}

pub fn inject_file(js_path: &Path, report: &mut InjectReport) -> Result<()> {
    debug!("Processing js file {}", js_path.display());

    let file =
        fs::read_to_string(js_path).context(format!("Failed to open {}", js_path.display()))?;

    if let Some(debug_id) = js::discover_debug_id(&file) {
        debug!("File {} was previously processed", js_path.display());
        report
            .previously_injected
            .push((js_path.to_path_buf(), debug_id));
        return Ok(());
    }

    let Some(sourcemap_url) = js::discover_sourcemaps_location(&file) else {
            debug!("File {} does not contain a sourcemap url", js_path.display());
            report.skipped.push(js_path.to_path_buf());
            return Ok(());
        };

    let sourcemap_path = js_path.with_file_name(sourcemap_url);

    if !sourcemap_path.exists() {
        debug!("Sourcemap file {} not found", sourcemap_path.display());
        report.missing_sourcemaps.push(js_path.to_path_buf());
        return Ok(());
    }

    let (debug_id, sourcemap_modified) = fixup_sourcemap(&sourcemap_path)
        .context(format!("Failed to process {}", sourcemap_path.display()))?;

    if sourcemap_modified {
        report.sourcemaps.push((sourcemap_path, debug_id));
    } else {
        report.skipped_sourcemaps.push((sourcemap_path, debug_id));
    }

    fixup_js_file(js_path, debug_id).context(format!("Failed to process {}", js_path.display()))?;

    report.injected.push((js_path.to_path_buf(), debug_id));

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
fn fixup_js_file(js_path: &Path, debug_id: DebugId) -> Result<()> {
    let js_lines = {
        let js_file = File::open(js_path)?;
        let js_file = BufReader::new(js_file);
        let js_lines: Result<Vec<_>, _> = js_file.lines().collect();
        js_lines?
    };

    let mut sourcemap_comment = None;
    let mut js_file = File::options().write(true).open(js_path)?;

    for line in js_lines.into_iter() {
        if line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=") {
            sourcemap_comment = Some(line);
            continue;
        }
        writeln!(js_file, "{line}")?;
    }

    let to_inject = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    writeln!(js_file)?;
    writeln!(js_file, "{to_inject}")?;
    writeln!(js_file, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    if let Some(sourcemap_comment) = sourcemap_comment {
        write!(js_file, "{sourcemap_comment}")?;
    }

    Ok(())
}

/// Fixes up a sourcemap file with a debug id.
///
/// If the file already contains a debug id under the `debug_id` key, it is left unmodified.
/// Otherwise, a fresh debug id is inserted under that key.
///
/// In either case, the value of the `debug_id` key is returned.
fn fixup_sourcemap(sourcemap_path: &Path) -> Result<(DebugId, bool)> {
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
            Ok((debug_id, false))
        }

        None => {
            let debug_id = DebugId::from_uuid(Uuid::new_v4());
            let id = serde_json::to_value(debug_id)?;
            map.insert(SOURCEMAP_DEBUGID_KEY.to_string(), id);

            serde_json::to_writer(&mut sourcemap_file, &sourcemap)?;
            Ok((debug_id, true))
        }
    }
}
