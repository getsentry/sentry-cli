use console::style;
use itertools::Itertools as _;
use regex::Regex;
use symbolic::common::{clean_path, join_path};

use std::fmt;
use std::io::{BufRead as _, Write as _};
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::Result;
use lazy_static::lazy_static;

use magic_string::{GenerateDecodedMapOptions, MagicString};
use sentry::types::DebugId;
use sourcemap::SourceMap;

const CODE_SNIPPET_TEMPLATE: &str = r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="__SENTRY_DEBUG_ID__")}catch(e){}}();"#;
const DEBUGID_PLACEHOLDER: &str = "__SENTRY_DEBUG_ID__";
const DEBUGID_COMMENT_PREFIX: &str = "//# debugId";

lazy_static! {
    // A regex that captures
    // 1. an optional initial hashbang,
    // 2. a block of line comments, block comments, and empty lines,
    // 3. and an optional `"use strict";` statement.`
    static ref PRE_INJECT_RE: Regex = Regex::new(
        r#"^(#!.*[\n\r])?(?:\s+|/\*(?:.|\r|\n)*?\*/|//.*[\n\r])*(?:"[^"]*";|'[^']*';[\n\r]?)?"#
    )
    .unwrap();
}

/// Regex that matches a "use [...]" directive at the beginning of a JS file,
/// possibly preceded by a block of line comments, block comments, and empty lines.
/// The use directive itself is captured in a named group `use_directive`.
static USE_DIRECTIVE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(?:\s+|/\*(?s:.)*?\*/|//.*[\n\r])*(?:(?<use_directive>"use [^"]*"|'use [^']*');?[\n\r]?)"#)
        .expect("this regex is valid")
});

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

/// Inject a debug ID code snippet and debug ID comment into compiled JS source.
///
/// This function takes the source as a &str and the debug ID to inject, and it returns
/// a String containing the injected source.
///
/// The code snippet is injected at the very beginning of the source (on its own line),
/// except when the source begins with a hashbang (`#!`), in which case, we inject
/// immediately after the hashbang. If the file starts with a "use ..." statement,
/// this statement is repeated in front of the injected snippet (on the same line).
/// The sourcemap for the original source must be adjusted by one line to account for
/// this injection.
///
/// The debug ID comment is added at the end of the file.
pub fn inject_at_start(compiled_source: &str, debug_id: DebugId) -> String {
    let (hashbang_portion, source_without_hasbang) = if compiled_source.starts_with("#!") {
        compiled_source.split_at(
            compiled_source
                .find("\n")
                .map(|i| i + 1)
                .unwrap_or(compiled_source.len()),
        )
    } else {
        ("", compiled_source)
    };

    let use_directive = USE_DIRECTIVE_REGEX
        .captures(source_without_hasbang)
        .map(|c| {
            format!(
                "{};",
                c.name("use_directive")
                    .expect("use directive always exists if regex matches")
                    .as_str()
            )
        })
        .unwrap_or_default();

    let code_snippet = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    let debug_id_comment = format!("{DEBUGID_COMMENT_PREFIX}={debug_id}");

    format!("{hashbang_portion}{use_directive}{code_snippet}\n{source_without_hasbang}\n{debug_id_comment}\n")
}

/// Fixes up a minified JS source file with a debug id.
///
/// This changes the source file in several ways:
/// 1. The source code snippet `<CODE_SNIPPET>[<debug_id>]` is inserted at the earliest possible position,
///    which is after an optional hashbang, followed by a block of comments, empty lines,
///    and an optional `"use […]";` or `'use […]';` pragma.
/// 2. A comment of the form `//# debugId=<debug_id>` is appended to the file.
///
/// This function returns a [`SourceMap`] that maps locations in the injected file
/// to their corresponding places in the original file.
#[deprecated(note = "New code should use `inject_start` instead")]
pub fn fixup_js_file(js_contents: &mut Vec<u8>, debug_id: DebugId) -> Result<SourceMap> {
    let contents = std::str::from_utf8(js_contents)?;

    let m = PRE_INJECT_RE
        .find(contents)
        .expect("regex is infallible")
        .range();

    let code_snippet = format!(
        "\n{}\n",
        CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string())
    );

    let debug_id_comment = format!("\n{DEBUGID_COMMENT_PREFIX}={debug_id}\n");

    let mut magic = MagicString::new(contents);

    magic
        .append_left(m.end as u32, &code_snippet)
        .unwrap()
        .append(&debug_id_comment)
        .unwrap();

    js_contents.clear();
    write!(js_contents, "{}", magic.to_string())?;

    let map = magic
        .generate_map(GenerateDecodedMapOptions {
            source: Some("pre_injection.js".to_owned()),
            include_content: true,
            ..Default::default()
        })
        .unwrap();

    let map = map.to_string().unwrap();

    Ok(SourceMap::from_slice(map.as_bytes()).unwrap())
}

/// Inject a minified JS source file with a debug id without changing mappings.
///
/// This changes the source file in several ways:
/// 1. The source code snippet `<CODE_SNIPPET>[<debug_id>]` is appended to the file.
/// 2. A comment of the form `//# debugId=<debug_id>` is appended to the file.
/// 3. The last source mapping comment (a comment starting with `//# sourceMappingURL=` or `//@ sourceMappingURL=`)
///    is moved to the very end of the file, after the debug id comment from 2.
///
/// This function is useful in cases where a source file's corresponding sourcemap is
/// not available. In such a case, [`inject_at_start`] (or the deprecated [`fixup_js_file`])
/// might mess up the mappings by inserting a line, with no opportunity to adjust the
/// sourcemap accordingly. However, in general it is desirable to insert the code snippet
/// as early as possible to make sure it runs even when an error is raised in the file.
pub fn inject_at_end(js_contents: &mut Vec<u8>, debug_id: DebugId) -> Result<()> {
    let mut js_lines = js_contents.lines().collect::<Result<Vec<_>, _>>()?;

    js_contents.clear();

    let sourcemap_comment_idx = js_lines.iter().rposition(|line| {
        line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=")
    });

    let sourcemap_comment = sourcemap_comment_idx.map(|idx| js_lines.remove(idx));

    for line in js_lines.into_iter() {
        writeln!(js_contents, "{line}")?;
    }

    let to_inject = CODE_SNIPPET_TEMPLATE.replace(DEBUGID_PLACEHOLDER, &debug_id.to_string());
    writeln!(js_contents, "{to_inject}")?;
    if let Some(sourcemap_comment) = sourcemap_comment {
        writeln!(js_contents, "{sourcemap_comment}")?;
    }
    writeln!(js_contents, "{DEBUGID_COMMENT_PREFIX}={debug_id}")?;

    Ok(())
}

/// Replaces a JS file's source mapping url with a new one.
///
/// Only the bottommost source mapping url comment will be updated. If there
/// are no source mapping url comments in the file, this is a no-op.
pub fn replace_sourcemap_url(js_contents: &mut Vec<u8>, new_url: &str) -> Result<()> {
    let js_lines = js_contents.lines().collect::<Result<Vec<_>, _>>()?;

    let Some(sourcemap_comment_idx) = js_lines.iter().rposition(|line| {
        line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=")
    }) else {
        return Ok(());
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

/// Ensures paths are always separated by `/` even on Windows
pub fn canonicalize_path_sep_to_unix(path: &str) -> String {
    path.replace(std::path::MAIN_SEPARATOR, "/")
}

/// Computes a normalized sourcemap URL from a source file's own URL und the relative URL of its sourcemap.
///
/// Roughly, this will combine a source URL of `some/dir/source.js` and a sourcemap URL of `path/to/source.min.js`
/// to `some/dir/path/to/source.min.js`, taking `..` and `.` path segments as well as absolute sourcemap URLs
/// into account.
///
/// Leading `./` segments will be preserved.
pub fn normalize_sourcemap_url(source_url: &str, sourcemap_url: &str) -> String {
    let canonicalized_source_url = canonicalize_path_sep_to_unix(source_url);
    let base_url = canonicalized_source_url
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("");

    let joined = join_path(base_url, sourcemap_url);
    let mut cutoff = 0;
    while joined[cutoff..].starts_with("./") {
        cutoff += 2;
    }

    // At the end we do a split by MAIN_SEPARATOR as everything operates with `/` in the code but
    // `clean_path` and `join_path` uses the system separator.
    canonicalize_path_sep_to_unix(&format!(
        "{}{}",
        &joined[..cutoff],
        clean_path(&joined[cutoff..])
    ))
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
            .is_some_and(|(x, y)| x == y)
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
    use std::str::FromStr as _;

    use sentry::types::DebugId;

    use crate::utils::fs::TempFile;

    use super::*;

    #[test]
    fn test_fixup_js_file() {
        let source = r#"//# sourceMappingURL=fake1
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else"#;

        let debug_id = DebugId::default();
        let mut source = Vec::from(source);

        #[expect(deprecated)]
        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"//# sourceMappingURL=fake1

!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else
//# debugId=00000000-0000-0000-0000-000000000000
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

        #[expect(deprecated)]
        fixup_js_file(&mut source, debug_id).unwrap();

        {
            let mut file = temp_file.open().unwrap();
            file.write_all(&source).unwrap();
        }

        let result = std::fs::read_to_string(temp_file.path()).unwrap();
        let expected = r#"//# sourceMappingURL=fake



!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
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
something else
//# debugId=00000000-0000-0000-0000-000000000000
"#;

        println!("{result}");
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

        #[expect(deprecated)]
        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict";
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
 rest of the line
'use strict';
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else
//# debugId=00000000-0000-0000-0000-000000000000
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

        #[expect(deprecated)]
        fixup_js_file(&mut source, debug_id).unwrap();

        let expected = r#"#!/bin/node
//# sourceMappingURL=fake1

  // some other comment
"use strict";
!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();
 rest of the line
(this.foo=this.bar||[]).push([[2],[function(e,t,n){"use strict"; […] }
some line
//# sourceMappingURL=fake2
//# sourceMappingURL=real
something else
//# debugId=00000000-0000-0000-0000-000000000000
"#;

        assert_eq!(std::str::from_utf8(&source).unwrap(), expected);
    }

    #[test]
    fn test_normalize_sourcemap_url() {
        // TODO: Enable the following test and make it pass
        // Linux allows having `\` in a file name but our path helpers,
        // specifically `join_path` and `clean_path` from `symbolic::common`,
        // do not use `std::path::MAIN_SEPARATOR` and instead tries to guess
        // the path style by the existence of a `\` in the path. This is
        // problematic because it can lead to incorrect path normalization.
        // assert_eq!(
        //     normalize_sourcemap_url("/foo/ba\\r/baz.js", "baz.js.map"),
        //     "/foo/ba\\r/baz.js.map"
        // );

        assert_eq!(
            normalize_sourcemap_url(
                &format!("foo{0}bar{0}baz.js", std::path::MAIN_SEPARATOR),
                &format!("..{0}.{0}baz.js.map", std::path::MAIN_SEPARATOR)
            ),
            "foo/baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url(
                &format!("foo{0}bar{0}baz.js", std::path::MAIN_SEPARATOR),
                ".././baz.js.map"
            ),
            "foo/baz.js.map"
        );

        assert_eq!(
            normalize_sourcemap_url(
                "foo/bar/baz.js",
                &format!("..{0}.{0}baz.js.map", std::path::MAIN_SEPARATOR)
            ),
            "foo/baz.js.map"
        );

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
        let candidates = &["./foo/baz/quux".to_owned(), "foo/baar/baz/quux".to_owned()];

        assert_eq!(
            find_matching_paths(candidates, expected),
            vec!["foo/baar/baz/quux"]
        );

        let candidates = &[
            "./foo/baz/quux".to_owned(),
            "foo/baar/baz/quux".to_owned(),
            "./foo/bar/baz/quux".to_owned(),
        ];

        assert_eq!(find_matching_paths(candidates, expected), vec![expected]);
    }

    #[test]
    fn test_find_matching_paths_ambiguous() {
        let expected = "./foo/bar/baz/quux";
        let candidates = &[
            "./foo/bar/baaz/quux".to_owned(),
            "foo/baar/baz/quux".to_owned(),
        ];

        assert_eq!(find_matching_paths(candidates, expected), candidates,);
    }

    #[test]
    fn test_find_matching_paths_filename() {
        let expected = "./foo/bar/baz/quux";
        let candidates = &[
            "./foo/bar/baz/nop".to_owned(),
            "foo/baar/baz/quux".to_owned(),
        ];

        assert_eq!(
            find_matching_paths(candidates, expected),
            ["foo/baar/baz/quux".to_owned()]
        );
    }

    #[test]
    fn test_find_matching_paths_sourcemaps() {
        let candidates = &[
            "./project/maps/index.js.map".to_owned(),
            "./project/maps/page/index.js.map".to_owned(),
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

    #[test]
    fn inject_debug_id_empty_source() {
        let source = "\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_just_source() {
        let source = "console.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "console.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_hashbang() {
        let source = "#!/usr/bin/env node\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Hashbang
                "#!/usr/bin/env node\n",
                // Injected debug ID code
                r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "console.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_use_strict() {
        let source = "\"use strict\";\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#""use strict";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "\"use strict\";\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_use_strict_on_same_line() {
        let source = "\"use strict\";console.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#""use strict";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "\"use strict\";console.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_comments() {
        let source = "// Some comment\n/* Block comment */\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "// Some comment\n/* Block comment */\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_with_custom_use_directive() {
        let source = "\"use custom\";\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#""use custom";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "\"use custom\";\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_comments_before_use_strict() {
        let source =
            "// Some comment\n/* Block comment */\n\"use strict\";\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#""use strict";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "// Some comment\n/* Block comment */\n\"use strict\";\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_single_quoted_use_strict() {
        let source = "'use strict';\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#"'use strict';!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "'use strict';\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_use_strict_no_semicolon() {
        let source = "\"use strict\"\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#""use strict";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "\"use strict\"\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_hashbang_comments_and_use_strict() {
        let source = "#!/usr/bin/env node\n// Some comment\n/* Block comment */\n\"use strict\";\nconsole.log('hello');\n";
        let debug_id = DebugId::default();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Hashbang
                "#!/usr/bin/env node\n",
                // Injected debug ID code
                r#""use strict";!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="00000000-0000-0000-0000-000000000000")}catch(e){}}();"#,
                "\n",
                // Original source code
                "// Some comment\n/* Block comment */\n\"use strict\";\nconsole.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=00000000-0000-0000-0000-000000000000\n"
            )
        );
    }

    #[test]
    fn inject_debug_id_with_custom_debug_id() {
        let source = "console.log('hello');\n";
        let debug_id = DebugId::from_str("12345678-1234-5678-1234-567812345678").unwrap();
        let result = inject_at_start(source, debug_id);
        assert_eq!(
            result,
            concat!(
                // Injected debug ID code
                r#"!function(){try{var e="undefined"!=typeof window?window:"undefined"!=typeof global?global:"undefined"!=typeof globalThis?globalThis:"undefined"!=typeof self?self:{},n=(new e.Error).stack;n&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[n]="12345678-1234-5678-1234-567812345678")}catch(e){}}();"#,
                "\n",
                // Original source code
                "console.log('hello');\n",
                // Debug ID comment
                "\n//# debugId=12345678-1234-5678-1234-567812345678\n"
            )
        );
    }
}
