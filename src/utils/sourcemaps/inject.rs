use itertools::Itertools;
use symbolic::common::{clean_path, join_path};

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
    let mut js_lines = js_lines?;

    js_contents.clear();

    let sourcemap_comment_idx = js_lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_idx, line)| {
            line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=")
        })
        .map(|(idx, _)| idx);

    let sourcemap_comment = sourcemap_comment_idx.map(|idx| js_lines.remove(idx));

    for line in js_lines.into_iter() {
        writeln!(js_contents, "{line}")?;
    }

    let to_inject = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    writeln!(js_contents, "{to_inject}")?;
    writeln!(js_contents, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    if let Some(sourcemap_comment) = sourcemap_comment {
        writeln!(js_contents, "{sourcemap_comment}")?;
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

            sourcemap_contents.clear();
            serde_json::to_writer(sourcemap_contents, &sourcemap)?;
            Ok((debug_id, true))
        }
    }
}

/// Computes a normalized sourcemap URL from a source file's own URL und the relative URL of its sourcemap.
///
/// Roughly, this will combine a source URL of `some/dir/source.js` and a sourcemap URL of `path/to/source.min.js`
/// to `some/dir/path/to/source.min.js`, taking `..` and `.` path segments as well as absolute sourcemap URLs
/// into account.
///
/// Leading `./` segments will be preserved.
pub fn normalize_sourcemap_url(source_url: &str, sourcemap_url: &str) -> String {
    let base_url = source_url
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("");

    let joined = join_path(base_url, sourcemap_url);
    let mut cutoff = 0;
    while joined[cutoff..].starts_with("./") {
        cutoff += 2;
    }

    format!("{}{}", &joined[..cutoff], clean_path(&joined[cutoff..]))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use sentry::types::DebugId;

    use crate::utils::fs::TempFile;

    use super::{fixup_js_file, fixup_sourcemap, normalize_sourcemap_url};

    #[test]
    fn test_fixup_sourcemap() {
        for sourcemap_path in &[
            "server/chunks/1.js.map",
            "server/edge-runtime-webpack.js.map",
            "server/pages/_document.js.map",
            "server/pages/asdf.js.map",
            "static/chunks/575-bb7d7e0e6de8d623.js.map",
            "static/chunks/app/client/page-d5742c254d9533f8.js.map",
            "static/chunks/pages/asdf-05b39167abbe433b.js.map",
        ] {
            let mut sourcemap_contents = std::fs::read(format!(
                "tests/integration/_fixtures/inject/{sourcemap_path}"
            ))
            .unwrap();

            assert!(
                sourcemap::decode_slice(&sourcemap_contents).is_ok(),
                "sourcemap is valid before injection"
            );

            fixup_sourcemap(&mut sourcemap_contents).unwrap();

            assert!(
                sourcemap::decode_slice(&sourcemap_contents).is_ok(),
                "sourcemap is valid after injection"
            );
        }
    }

    #[test]
    fn test_fixup_js_file() {
        let source = r#"//# sourceMappingURL=fake1
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else"#;

        let debug_id = DebugId::default();

        let mut source = Vec::from(source);

        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"//# sourceMappingURL=fake1
some line
//# sourceMappingURL=fake2
something else
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}()
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        assert_eq!(std::str::from_utf8(&source).unwrap(), expected);
    }

    #[test]
    fn test_fixup_js_file_fs_roundtrip() {
        let source = r#"//# sourceMappingURL=fake1


some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else"#;

        let temp_file = TempFile::create().unwrap();
        {
            let mut file = temp_file.open().unwrap();
            write!(file, "{source}").unwrap();
        }

        let debug_id = DebugId::default();
        let mut source = std::fs::read(temp_file.path()).unwrap();

        fixup_js_file(&mut source, debug_id).unwrap();

        {
            let mut file = temp_file.open().unwrap();
            file.write_all(&source).unwrap();
        }

        let result = std::fs::read_to_string(temp_file.path()).unwrap();
        let expected = r#"//# sourceMappingURL=fake1


some line
//# sourceMappingURL=fake2
something else
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}()
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        println!("{}", result);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_normalize_sourcemap_url() {
        assert_eq!(
            normalize_sourcemap_url("foo/bar/baz.js", "baz.js.map"),
            "foo/bar/baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url("baz.js", "baz.js.map"),
            "baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url("foo/bar/baz.js", ".././baz.js.map"),
            "foo/baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url("baz.js", ".././baz.js.map"),
            "../baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url("foo/bar/baz.js", "/quux/baz.js.map"),
            "/quux/baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url("././.foo/bar/baz.js", "../quux/baz.js.map"),
            "././.foo/quux/baz.js.map"
        );
    }
}
