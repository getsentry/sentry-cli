use console::style;
use itertools::Itertools;
use regex::Regex;
use symbolic::common::{clean_path, join_path};

use std::fmt;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use log::debug;
use sentry::types::DebugId;
use serde_json::Value;

const CODE_SNIPPET_TEMPLATE: &str = r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="__SENTRY_DEBUG_ID__")}catch(e){}}();"#;
const DEBUGID_PLACEHOLDER: &str = "__SENTRY_DEBUG_ID__";
const DEBUGID_COMMENT_PREFIX: &str = "//# debugId";

lazy_static! {
    static ref USE_PRAGMA_RE: Regex = Regex::new(r#"^"use \w+";|^'use \w+';"#).unwrap();
}

fn print_section_with_debugid(
    f: &mut fmt::Formatter<'_>,
    title: &str,
    data: &[(PathBuf, DebugId)],
) -> fmt::Result {
    print_section_title(f, title)?;
    for (path, debug_id) in data.iter().sorted_by_key(|x| &x.0) {
        writeln!(f, "    {debug_id} - {}", path.display())?;
    }
    Ok(())
}

fn print_section_title(f: &mut fmt::Formatter<'_>, title: &str) -> fmt::Result {
    writeln!(f, "  {}", style(title).yellow().bold())
}

#[derive(Debug, Clone, Default)]
pub struct InjectReport {
    pub injected: Vec<(PathBuf, DebugId)>,
    pub previously_injected: Vec<(PathBuf, DebugId)>,
    pub sourcemaps: Vec<(PathBuf, DebugId)>,
    pub skipped_sourcemaps: Vec<(PathBuf, DebugId)>,
}

impl InjectReport {
    pub fn is_empty(&self) -> bool {
        self.injected.is_empty()
            && self.previously_injected.is_empty()
            && self.sourcemaps.is_empty()
            && self.skipped_sourcemaps.is_empty()
    }
}

impl fmt::Display for InjectReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\n{}",
            style("Source Map Debug ID Injection Report").dim().bold()
        )?;

        if !self.injected.is_empty() {
            print_section_with_debugid(
                f,
                "Modified: The following source files have been modified to have debug ids",
                &self.injected,
            )?;
        }

        if !self.sourcemaps.is_empty() {
            print_section_with_debugid(
                f,
                "Modified: The following sourcemap files have been modified to have debug ids",
                &self.sourcemaps,
            )?;
        }

        if !self.previously_injected.is_empty() {
            print_section_with_debugid(
                f,
                "Ignored: The following source files already have debug ids",
                &self.previously_injected,
            )?;
        }

        if !self.skipped_sourcemaps.is_empty() {
            print_section_with_debugid(
                f,
                "Ignored: The following sourcemap files already have debug ids",
                &self.skipped_sourcemaps,
            )?;
        }

        Ok(())
    }
}

/// Fixes up a minified JS source file with a debug id.
///
/// This changes the source file in several ways:
/// 1. The source code snippet
/// `<CODE_SNIPPET>[<debug_id>]`
/// is inserted at the earliest possible position, which is after an
/// optional hashbang, followed by a
/// block of comments, empty lines, and `"use […]";` or `'use […]';` pragmas.
/// 2. A comment of the form `//# debugId=<debug_id>` is appended to the file.
/// 3. The last source mapping comment (a comment starting with
/// `//# sourceMappingURL=` or `//@ sourceMappingURL=`) is moved to
/// the very end of the file, after the debug id comment from 2.
///
/// This function will naturally mess with the correspondence between a source file
/// and its sourcemap. Use [`insert_empty_mapping`] on the sourcemap to fix this.
/// # Example
/// ```
/// let file = "
/// // a
/// // comment
/// // block
///
/// // another
/// // comment
/// // block
///
/// 'use strict';
/// function t(t) {
///   return '[object Object]' === Object.prototype.toString.call(t);
/// }
/// //# sourceMappingURL=/path/to/sourcemap
/// ";
///
/// let mut file = file.as_bytes().to_vec();
/// fixup_js_file(&mut file, DebugId::default()).unwrap();
///
/// assert_eq!(
///     std::str::from_utf8(&file).unwrap(),
///     r#"
/// // a
/// // comment
/// // block
///
/// // another
/// // comment
/// // block
///
/// 'use strict';
/// !function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
/// function t(t) {
///   return '[object Object]' === Object.prototype.toString.call(t);
/// }
/// //# debugId=00000000-0000-0000-0000-000000000000
/// //# sourceMappingURL=/path/to/sourcemap
/// "#
/// );
/// ```
pub fn fixup_js_file(js_contents: &mut Vec<u8>, debug_id: DebugId) -> Result<()> {
    let mut js_lines = js_contents.lines().collect::<Result<Vec<_>, _>>()?;

    js_contents.clear();

    // Find the last source mapping URL comment, it's the only one that matters
    let sourcemap_comment_idx = js_lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_idx, line)| {
            line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=")
        })
        .map(|(idx, _)| idx);

    let sourcemap_comment = sourcemap_comment_idx.map(|idx| js_lines.remove(idx));

    let mut js_lines = js_lines.into_iter().peekable();

    // Handle initial hashbang
    if let Some(hashbang) = js_lines.next_if(|line| line.trim().starts_with("#!")) {
        writeln!(js_contents, "{hashbang}")?;
    }

    // Write comments and empty lines at the start back to the file
    while let Some(comment_or_empty) =
        js_lines.next_if(|line| line.trim().is_empty() || line.trim().starts_with("//"))
    {
        writeln!(js_contents, "{comment_or_empty}")?;
    }

    // Write use statements back to the file
    while let Some(use_pragma) = js_lines.next_if(|line| USE_PRAGMA_RE.is_match(line)) {
        writeln!(js_contents, "{use_pragma}")?;
    }

    // Inject the code snippet
    let to_inject = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    writeln!(js_contents, "{to_inject}")?;

    // Write other lines
    for line in js_lines {
        writeln!(js_contents, "{line}")?;
    }

    // Write the debug id comment
    writeln!(js_contents, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    // Lastly, write the source mapping URL comment, if there was one
    if let Some(sourcemap_comment) = sourcemap_comment {
        writeln!(js_contents, "{sourcemap_comment}")?;
    }

    Ok(())
}

/// Fixes up a minified JS source file with a debug id without messing with mappings.
///
/// This changes the source file in several ways:
/// 1. The source code snippet
/// `<CODE_SNIPPET>[<debug_id>]` is appended to the file.
/// 2. A comment of the form `//# debugId=<debug_id>` is appended to the file.
/// 3. The last source mapping comment (a comment starting with
/// `//# sourceMappingURL=` or `//@ sourceMappingURL=`) is moved to
/// the very end of the file, after the debug id comment from 2.
///
/// This function is useful in cases where a source file's corresponding sourcemap is
/// not available. In such a case, [`fixup_js_file`] might mess up the mappings by inserting
/// a line, with no opportunity to adjust the sourcemap accordingly. However, in general
/// it is desirable to insert the code snippet as early as possible to make sure it runs
/// even when an error is raised in the file.
pub fn fixup_js_file_end(js_contents: &mut Vec<u8>, debug_id: DebugId) -> Result<()> {
    let mut js_lines = js_contents.lines().collect::<Result<Vec<_>, _>>()?;

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

/// Replaces a JS file's source mapping url with a new one.
///
/// Only the bottommost source mapping url comment will be updated. If there
/// are no source mapping url comments in the file, this is a no-op.
pub fn replace_sourcemap_url(js_contents: &mut Vec<u8>, new_url: &str) -> Result<()> {
    let js_lines = js_contents.lines().collect::<Result<Vec<_>, _>>()?;

    let sourcemap_comment_idx = match js_lines.iter().enumerate().rev().find(|(_idx, line)| {
        line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=")
    }) {
        Some((idx, _)) => idx,
        None => return Ok(()),
    };

    js_contents.clear();

    for line in &js_lines[0..sourcemap_comment_idx] {
        writeln!(js_contents, "{line}")?;
    }

    writeln!(js_contents, "//# sourceMappingURL={new_url}")?;

    for line in &js_lines[sourcemap_comment_idx + 1..] {
        writeln!(js_contents, "{line}")?;
    }

    Ok(())
}

/// Generates a debug ID from bytes.
pub fn debug_id_from_bytes_hashed(bytes: &[u8]) -> DebugId {
    let mut hash = sha1_smol::Sha1::new();
    hash.update(bytes);
    let mut sha1_bytes = [0u8; 16];
    sha1_bytes.copy_from_slice(&hash.digest().bytes()[..16]);
    DebugId::from_uuid(uuid::Builder::from_sha1_bytes(sha1_bytes).into_uuid())
}

/// Fixes up a sourcemap file with a debug id.
///
/// If the file already contains a debug id under the `debug_id` key, it is left unmodified.
/// Otherwise, a fresh debug id is inserted under that key.
///
/// In either case, the value of the `debug_id` key is returned.
pub fn fixup_sourcemap(sourcemap_contents: &mut Vec<u8>) -> Result<(DebugId, bool)> {
    match sourcemap::decode_slice(sourcemap_contents).context("Invalid sourcemap")? {
        sourcemap::DecodedMap::Regular(mut sm) => {
            if let Some(debug_id) = sm.get_debug_id() {
                debug!("Sourcemap already has a debug id");
                Ok((debug_id, false))
            } else {
                let debug_id = debug_id_from_bytes_hashed(sourcemap_contents);
                sm.set_debug_id(Some(debug_id));

                sourcemap_contents.clear();
                sm.to_writer(sourcemap_contents)?;
                Ok((debug_id, true))
            }
        }
        sourcemap::DecodedMap::Hermes(mut smh) => {
            if let Some(debug_id) = smh.get_debug_id() {
                debug!("Sourcemap already has a debug id");
                Ok((debug_id, false))
            } else {
                let debug_id = debug_id_from_bytes_hashed(sourcemap_contents);
                smh.set_debug_id(Some(debug_id));

                sourcemap_contents.clear();
                smh.to_writer(sourcemap_contents)?;
                Ok((debug_id, true))
            }
        }
        sourcemap::DecodedMap::Index(_) => {
            bail!("DebugId injection is not supported for sourcemap indexes")
        }
    }
}

/// This adds an empty mapping at the start of a sourcemap.
///
/// This is used to adjust a sourcemap when the corresponding source file has a
/// new line injected near the top (see [`fixup_js_file`]).
pub fn insert_empty_mapping(sourcemap_contents: &mut Vec<u8>) -> Result<()> {
    let mut sourcemap: Value = serde_json::from_slice(sourcemap_contents)?;

    let Some(map) = sourcemap.as_object_mut() else {
        bail!("Invalid sourcemap");
    };

    let Some(mappings) = map.get_mut("mappings") else {
        bail!("Invalid sourcemap");
    };

    let Value::String(mappings) = mappings else {
        bail!("Invalid sourcemap");
    };

    // Insert empty mapping at the start
    *mappings = format!(";{mappings}");

    sourcemap_contents.clear();
    serde_json::to_writer(sourcemap_contents, &sourcemap)?;

    Ok(())
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

/// Returns a list of those paths among `candidate_paths` that differ from `expected_path` in
/// at most one segment (modulo `.` segments).
///
/// The differing segment cannot be the last one (i.e., the filename).
///
/// If `expected_path` occurs among the `candidate_paths`, no other paths will be returned since
/// that is considered a unique best match.
///
/// The intended usecase is finding sourcemaps even if they reside in a different directory; see
/// the `test_find_matching_paths_sourcemaps` test for a minimal example.
pub fn find_matching_paths(candidate_paths: &[String], expected_path: &str) -> Vec<String> {
    let mut matches = Vec::new();
    for candidate in candidate_paths {
        let mut expected_segments = expected_path
            .split('/')
            .filter(|&segment| segment != ".")
            .peekable();
        let mut candidate_segments = candidate
            .split('/')
            .filter(|&segment| segment != ".")
            .peekable();

        // If there is a candidate that is exactly equal to the goal path,
        // return only that one.
        if Iterator::eq(candidate_segments.clone(), expected_segments.clone()) {
            return vec![candidate.clone()];
        }

        // Iterate through both paths and discard segments so long as they are equal.
        while candidate_segments
            .peek()
            .zip(expected_segments.peek())
            .map_or(false, |(x, y)| x == y)
        {
            candidate_segments.next();
            expected_segments.next();
        }

        // The next segments (if there are any left) must be where the paths disagree.
        candidate_segments.next();
        expected_segments.next();

        // The rest of both paths must agree and be nonempty, so at least the filenames definitely
        // must agree.
        if candidate_segments.peek().is_some()
            && Iterator::eq(candidate_segments, expected_segments)
        {
            matches.push(candidate.clone());
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use sentry::types::DebugId;

    use crate::utils::fs::TempFile;

    use super::*;

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
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
some line
//# sourceMappingURL=fake2
something else
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        assert_eq!(std::str::from_utf8(&source).unwrap(), expected);
    }

    #[test]
    fn test_fixup_js_file_fs_roundtrip() {
        let source = r#"//# sourceMappingURL=fake


some line
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
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
        let expected = r#"//# sourceMappingURL=fake


!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
some line
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
//# sourceMappingURL=fake
something else
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        println!("{}", result);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_fixup_js_file_use_strict() {
        let source = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict"; rest of the line
'use strict';
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else"#;

        let debug_id = DebugId::default();

        let mut source = Vec::from(source);

        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict"; rest of the line
'use strict';
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
some line
//# sourceMappingURL=fake2
something else
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        assert_eq!(std::str::from_utf8(&source).unwrap(), expected);
    }

    #[test]
    fn test_fixup_js_file_fake_use_strict() {
        let source = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict"; rest of the line
(this.foo=this.bar||[]).push([[2],[function(e,t,n){"use strict"; […] }
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else"#;

        let debug_id = DebugId::default();

        let mut source = Vec::from(source);

        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict"; rest of the line
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof self?self:{},n=(new Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
(this.foo=this.bar||[]).push([[2],[function(e,t,n){"use strict"; […] }
some line
//# sourceMappingURL=fake2
something else
//# debugId=00000000-0000-0000-0000-000000000000
//# sourceMappingURL=real
"#;

        assert_eq!(std::str::from_utf8(&source).unwrap(), expected);
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

    #[test]
    fn test_replace_sourcemap_url() {
        let js_contents = r#"
//# sourceMappingURL=not this one
some text
//@ sourceMappingURL=not this one either
//# sourceMappingURL=this one
more text
"#;
        let mut js_contents = Vec::from(js_contents);

        replace_sourcemap_url(&mut js_contents, "new url").unwrap();

        let expected = r#"
//# sourceMappingURL=not this one
some text
//@ sourceMappingURL=not this one either
//# sourceMappingURL=new url
more text
"#;
        assert_eq!(std::str::from_utf8(&js_contents).unwrap(), expected);
    }

    #[test]
    fn test_find_matching_paths_unique() {
        let expected = "./foo/bar/baz/quux";
        let candidates = &[
            "./foo/baz/quux".to_string(),
            "foo/baar/baz/quux".to_string(),
        ];

        assert_eq!(
            find_matching_paths(candidates, expected),
            vec!["foo/baar/baz/quux"]
        );

        let candidates = &[
            "./foo/baz/quux".to_string(),
            "foo/baar/baz/quux".to_string(),
            "./foo/bar/baz/quux".to_string(),
        ];

        assert_eq!(find_matching_paths(candidates, expected), vec![expected]);
    }

    #[test]
    fn test_find_matching_paths_ambiguous() {
        let expected = "./foo/bar/baz/quux";
        let candidates = &[
            "./foo/bar/baaz/quux".to_string(),
            "foo/baar/baz/quux".to_string(),
        ];

        assert_eq!(find_matching_paths(candidates, expected), candidates,);
    }

    #[test]
    fn test_find_matching_paths_filename() {
        let expected = "./foo/bar/baz/quux";
        let candidates = &[
            "./foo/bar/baz/nop".to_string(),
            "foo/baar/baz/quux".to_string(),
        ];

        assert_eq!(
            find_matching_paths(candidates, expected),
            ["foo/baar/baz/quux".to_string()]
        );
    }

    #[test]
    fn test_find_matching_paths_sourcemaps() {
        let candidates = &[
            "./project/maps/index.js.map".to_string(),
            "./project/maps/page/index.js.map".to_string(),
        ];

        assert_eq!(
            find_matching_paths(candidates, "project/code/index.js.map"),
            &["./project/maps/index.js.map"]
        );

        assert_eq!(
            find_matching_paths(candidates, "project/code/page/index.js.map"),
            &["./project/maps/page/index.js.map"]
        );
    }
}
