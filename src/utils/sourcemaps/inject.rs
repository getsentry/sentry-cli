use itertools::Itertools;

use std::fmt;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};
use log::debug;
use sentry::types::DebugId;
use serde_json::Value;
use uuid::Uuid;

const CODE_SNIPPET_TEMPLATE: &str = r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="__SENTRY_DEBUG_ID__")}catch(e){}}()"#;
const DEBUGID_PLACEHOLDER: &str = "__SENTRY_DEBUG_ID__";
const SOURCEMAP_DEBUGID_KEY: &str = "debug_id";
const DEBUGID_COMMENT_PREFIX: &str = "//# debugId";

#[derive(Debug, Clone, Default)]
pub struct InjectReport {
    pub injected: Vec<(PathBuf, DebugId)>,
    pub previously_injected: Vec<(PathBuf, DebugId)>,
    pub skipped: Vec<PathBuf>,
    pub missing_sourcemaps: Vec<PathBuf>,
    pub sourcemaps: Vec<(PathBuf, DebugId)>,
    pub skipped_sourcemaps: Vec<(PathBuf, DebugId)>,
}

impl InjectReport {
    pub fn is_empty(&self) -> bool {
        self.injected.is_empty()
            && self.previously_injected.is_empty()
            && self.skipped.is_empty()
            && self.missing_sourcemaps.is_empty()
            && self.sourcemaps.is_empty()
            && self.skipped_sourcemaps.is_empty()
    }
}

impl fmt::Display for InjectReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.injected.is_empty() {
            writeln!(
                f,
                "Modified: The following source files have been modified to have debug ids"
            )?;
            for (path, debug_id) in self.injected.iter().sorted_by_key(|x| &x.0) {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.sourcemaps.is_empty() {
            writeln!(
                f,
                "\nModified: The following sourcemap files have been modified to have debug ids"
            )?;
            for (path, debug_id) in self.sourcemaps.iter().sorted_by_key(|x| &x.0) {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.previously_injected.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files already have debug ids"
            )?;
            for (path, debug_id) in self.previously_injected.iter().sorted_by_key(|x| &x.0) {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.skipped_sourcemaps.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following sourcemap files already have debug ids"
            )?;
            for (path, debug_id) in self.skipped_sourcemaps.iter().sorted_by_key(|x| &x.0) {
                writeln!(f, "  - {debug_id} - {}", path.display())?;
            }
        }

        if !self.skipped.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files don't have sourcemap references "
            )?;
            for path in self.skipped.iter().sorted() {
                writeln!(f, "  - {}", path.display())?;
            }
        }

        if !self.missing_sourcemaps.is_empty() {
            writeln!(
                f,
                "\nIgnored: The following source files refer to sourcemaps that couldn't be found"
            )?;
            for path in self.missing_sourcemaps.iter().sorted() {
                writeln!(f, "  - {}", path.display())?;
            }
        }

        Ok(())
    }
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
///
/// Moreover, if a `sourceMappingURL` comment exists in the file, it is moved to the very end.
pub fn fixup_js_file(js_contents: &mut Vec<u8>, debug_id: DebugId) -> Result<()> {
    let js_lines: Result<Vec<String>, _> = js_contents.lines().collect();

    let mut sourcemap_comment = None;
    js_contents.clear();

    for line in js_lines?.into_iter() {
        if line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=") {
            sourcemap_comment = Some(line);
            continue;
        }
        writeln!(js_contents, "{line}")?;
    }

    let to_inject = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    writeln!(js_contents)?;
    writeln!(js_contents, "{to_inject}")?;
    writeln!(js_contents, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    if let Some(sourcemap_comment) = sourcemap_comment {
        write!(js_contents, "{sourcemap_comment}")?;
    }

    Ok(())
}

/// Fixes up a sourcemap file with a debug id.
///
/// If the file already contains a debug id under the `debug_id` key, it is left unmodified.
/// Otherwise, a fresh debug id is inserted under that key.
///
/// In either case, the value of the `debug_id` key is returned.
pub fn fixup_sourcemap(sourcemap_contents: &mut Vec<u8>) -> Result<(DebugId, bool)> {
    let mut sourcemap: Value = serde_json::from_slice(sourcemap_contents)?;

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

            serde_json::to_writer(sourcemap_contents, &sourcemap)?;
            Ok((debug_id, true))
        }
    }
}
