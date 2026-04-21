#![expect(clippy::unwrap_used, reason = "contains legacy code which uses unwrap")]

use crate::config::Config;
use crate::utils::args::ArgExt as _;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::SourceFile;
use crate::utils::fs::path_as_url;
use crate::utils::source_bundle::{self, BundleContext};
use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::{debug, warn};
use sentry::types::DebugId;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::sync::Arc;
use symbolic::debuginfo::sourcebundle::SourceFileType;

const JVM_EXTENSIONS: &[&str] = &[
    "java", "kt", "scala", "sc", "groovy", "gvy", "gy", "gsh", "clj", "cljc",
];

/// Directory names that mark the root of a JVM source set (i.e. the parent of
/// the package hierarchy). Matches the Gradle/Maven convention
/// `src/<sourceset>/<lang>/<package>/...`.
const SOURCE_SET_LANGS: &[&str] = &["java", "kotlin", "scala", "groovy"];

/// Strips the `[<module>/]src/<sourceset>/<lang>/` prefix from a relative source
/// path so the remaining portion matches what Symbolicator looks up by URL
/// (e.g. `io/sentry/android/core/ANRWatchDog.java`). This is needed because
/// JVM stack traces reference classes by their package path, with no knowledge
/// of the containing Gradle module or source-set layout on disk.
///
/// Returns the path unchanged if no `src/<sourceset>/<lang>/` segment is found.
fn strip_source_set_prefix(relative_path: &Path) -> PathBuf {
    let mut iter = relative_path.components();
    let mut src_two_back = false;
    let mut src_one_back = false;
    while let Some(curr) = iter.next() {
        let curr_is_lang = curr
            .as_os_str()
            .to_str()
            .is_some_and(|s| SOURCE_SET_LANGS.contains(&s));
        if src_two_back && curr_is_lang {
            return iter.collect();
        }
        src_two_back = src_one_back;
        src_one_back = curr.as_os_str() == "src";
    }
    relative_path.to_path_buf()
}

/// Builds the Symbolicator-compatible URL for a relative source path
/// (e.g. `~/io/sentry/android/core/ANRWatchDog.jvm`).
fn build_source_url(relative_path: &Path) -> String {
    let package_path = strip_source_set_prefix(relative_path);
    let package_path_jvm_ext = package_path.with_extension("jvm");
    format!("~/{}", path_as_url(&package_path_jvm_ext))
}

/// Safe to exclude globally — can never be valid JVM package names.
const SAFE_EXCLUDES: &[&str] = &[
    ".cxx",
    ".eclipse",
    ".fleet",
    ".gradle",
    ".idea",
    ".kotlin",
    ".mvn",
    ".settings",
    ".vscode",
    "node_modules",
];

/// Common build output dirs that could also be valid JVM package names
/// (e.g. `com.example.build`). Only excluded outside of `src/` directories.
const AMBIGUOUS_EXCLUDES: &[&str] = &["bin", "build", "out", "target"];

/// Checks *all* ambiguous directories in the path and excludes if any of them
/// is not under a `src/` ancestor. Handles nested cases like
/// `build/src/main/java/com/example/target/Foo.java` — inner `target` is under
/// `src`, but outer `build` is not, so the file is excluded.
fn is_in_ambiguous_build_dir(relative_path: &Path) -> bool {
    for ancestor in relative_path.ancestors() {
        let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if AMBIGUOUS_EXCLUDES.contains(&name) {
            // Check if any ancestor *above* this directory is named "src".
            let has_src_above = ancestor
                .ancestors()
                .skip(1) // skip the ambiguous dir itself
                .any(|a| a.file_name() == Some(OsStr::new("src")));
            if !has_src_above {
                return true;
            }
        }
    }
    false
}

pub fn make_command(command: Command) -> Command {
    command
        .hide(true) // experimental for now
        .about(
            "Create a source bundle for the given JVM based source files (e.g. Java, Kotlin, ...)",
        )
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new())
                .help("The directory containing source files to bundle."),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new())
                .help("The path to the output folder."),
        )
        .arg(
            Arg::new("debug_id")
                .long("debug-id")
                .value_name("UUID")
                .required(true)
                .value_parser(DebugId::from_str)
                .help("Debug ID (UUID) to use for the source bundle."),
        )
        .arg(
            Arg::new("exclude")
                .long("exclude")
                .value_name("PATTERN")
                .action(ArgAction::Append)
                .help(
                    "Glob pattern to exclude files/directories. Can be repeated. \
                     By default, common build output and IDE directories are excluded \
                     (build, .gradle, target, .idea, .vscode, out, bin, etc.).",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();

    let context = BundleContext::new(&org).with_projects(project.as_slice());
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let output_path = matches.get_one::<PathBuf>("output").unwrap();
    let debug_id = matches.get_one::<DebugId>("debug_id").unwrap();
    let out = output_path.join(format!("{debug_id}.zip"));

    if !path.exists() {
        bail!("Given path does not exist: {}", path.display())
    }

    if !path.is_dir() {
        bail!("Given path is not a directory: {}", path.display())
    }

    if !output_path.exists() {
        fs::create_dir_all(output_path).context(format!(
            "Failed to create output directory {}",
            output_path.display()
        ))?;
    }

    let all_excludes = SAFE_EXCLUDES
        .iter()
        .copied()
        .chain(
            matches
                .get_many::<String>("exclude")
                .into_iter()
                .flatten()
                .map(|s| s.as_str()),
        )
        .map(|v| format!("!{v}"));

    let sources = ReleaseFileSearch::new(path.clone())
        .extensions(JVM_EXTENSIONS.iter().copied())
        .ignores(all_excludes)
        .respect_ignores(true)
        .collect_files()?;

    // Android build variants commonly contribute the same FQCN from
    // different source sets (e.g. `src/main/` and `src/debug/` both
    // defining `com.example.Foo`). After stripping, both map to the same
    // URL — drop all but the first and tell the user how to scope the bundle.
    let mut seen_urls: HashMap<String, PathBuf> = HashMap::new();
    let files: Vec<SourceFile> = sources
        .into_iter()
        .filter_map(|source| {
            let local_path = source.path.strip_prefix(&source.base_path).unwrap();
            if is_in_ambiguous_build_dir(local_path) {
                debug!("excluding (build output): {}", source.path.display());
                return None;
            }
            let url = build_source_url(local_path);

            match seen_urls.entry(url) {
                Entry::Occupied(existing) => {
                    warn!(
                        "URL collision on {}: skipping '{}' (already bundled from '{}'). \
                         Use --exclude to drop the unwanted source set \
                         (e.g. --exclude='**src/debug/**').",
                        existing.key(),
                        source.path.display(),
                        existing.get().display(),
                    );
                    None
                }
                Entry::Vacant(slot) => {
                    let url = slot.key().clone();
                    slot.insert(source.path.clone());
                    Some(SourceFile {
                        url,
                        path: source.path,
                        contents: Arc::new(source.contents),
                        ty: SourceFileType::Source,
                        headers: BTreeMap::new(),
                        messages: vec![],
                        already_uploaded: false,
                    })
                }
            }
        })
        .collect();

    let tempfile = source_bundle::build(context, files, Some(*debug_id))
        .context("Unable to create source bundle")?;

    fs::copy(tempfile.path(), &out).context("Unable to write source bundle")?;
    println!("Created {}", out.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_excludes_build_output_at_module_root() {
        assert!(is_in_ambiguous_build_dir(Path::new(
            "app/build/generated/Foo.java"
        )));
        assert!(is_in_ambiguous_build_dir(Path::new(
            "build/generated/Foo.java"
        )));
        assert!(is_in_ambiguous_build_dir(Path::new(
            "module/target/classes/Foo.java"
        )));
        assert!(is_in_ambiguous_build_dir(Path::new("bin/Foo.class")));
        assert!(is_in_ambiguous_build_dir(Path::new(
            "out/production/Foo.java"
        )));
    }

    #[test]
    fn test_keeps_source_packages_under_src() {
        assert!(!is_in_ambiguous_build_dir(Path::new(
            "src/main/java/com/example/build/Builder.java"
        )));
        assert!(!is_in_ambiguous_build_dir(Path::new(
            "app/src/main/java/com/example/target/Target.java"
        )));
        assert!(!is_in_ambiguous_build_dir(Path::new(
            "src/main/kotlin/com/example/out/Output.kt"
        )));
    }

    #[test]
    fn test_excludes_build_dir_containing_src() {
        // build/src/... should still be excluded — src is *inside* build, not above it
        assert!(is_in_ambiguous_build_dir(Path::new(
            "build/src/main/java/Foo.java"
        )));
        assert!(is_in_ambiguous_build_dir(Path::new(
            "app/build/src/generated/Foo.java"
        )));
    }

    #[test]
    fn test_excludes_nested_ambiguous_dirs_under_build() {
        // build/src/.../target/ — inner `target` is under src, but outer `build` is not
        assert!(is_in_ambiguous_build_dir(Path::new(
            "build/src/main/java/com/example/target/Foo.java"
        )));
        assert!(is_in_ambiguous_build_dir(Path::new(
            "target/src/main/java/com/example/out/Foo.java"
        )));
    }

    #[test]
    fn test_strip_source_set_prefix_drops_module_and_source_set() {
        assert_eq!(
            strip_source_set_prefix(Path::new(
                "sentry-android-core/src/main/java/io/sentry/android/core/ANRWatchDog.java"
            )),
            Path::new("io/sentry/android/core/ANRWatchDog.java")
        );
        assert_eq!(
            strip_source_set_prefix(Path::new("src/main/kotlin/com/example/Foo.kt")),
            Path::new("com/example/Foo.kt")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_kt_under_java_source_set() {
        // Mixed Java/Kotlin projects commonly place .kt files under src/main/java/
        // — stripping is driven by the directory name, not the file extension.
        assert_eq!(
            strip_source_set_prefix(Path::new("src/main/java/com/example/Foo.kt")),
            Path::new("com/example/Foo.kt")
        );
        assert_eq!(
            strip_source_set_prefix(Path::new(
                "app/src/main/java/io/sentry/android/core/ANRWatchDog.kt"
            )),
            Path::new("io/sentry/android/core/ANRWatchDog.kt")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_handles_nested_modules() {
        assert_eq!(
            strip_source_set_prefix(Path::new(
                "sentry-opentelemetry/sentry-opentelemetry-agent/src/main/java/io/sentry/opentelemetry/agent/Foo.java"
            )),
            Path::new("io/sentry/opentelemetry/agent/Foo.java")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_handles_android_variants() {
        assert_eq!(
            strip_source_set_prefix(Path::new("app/src/debug/java/com/example/Foo.java")),
            Path::new("com/example/Foo.java")
        );
        assert_eq!(
            strip_source_set_prefix(Path::new("lib/src/release/kotlin/com/example/Bar.kt")),
            Path::new("com/example/Bar.kt")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_supports_scala_and_groovy() {
        assert_eq!(
            strip_source_set_prefix(Path::new("mod/src/main/scala/com/example/Foo.scala")),
            Path::new("com/example/Foo.scala")
        );
        assert_eq!(
            strip_source_set_prefix(Path::new("mod/src/main/groovy/com/example/Foo.groovy")),
            Path::new("com/example/Foo.groovy")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_handles_default_package() {
        assert_eq!(
            strip_source_set_prefix(Path::new("src/main/java/NoPackage.java")),
            Path::new("NoPackage.java")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_falls_back_when_no_match() {
        // No `src/<sourceset>/<lang>/` triplet — path is returned unchanged.
        assert_eq!(
            strip_source_set_prefix(Path::new("sources/com/example/Foo.java")),
            Path::new("sources/com/example/Foo.java")
        );
        assert_eq!(
            strip_source_set_prefix(Path::new("Foo.java")),
            Path::new("Foo.java")
        );
    }

    #[test]
    fn test_strip_source_set_prefix_does_not_match_package_named_like_lang() {
        // `kotlin` as a package name (under `src/main/java/`) must not be
        // mistaken for the source-set language dir.
        assert_eq!(
            strip_source_set_prefix(Path::new("src/main/java/com/example/kotlin/Foo.java")),
            Path::new("com/example/kotlin/Foo.java")
        );
    }

    #[test]
    fn test_keeps_files_without_ambiguous_dirs() {
        assert!(!is_in_ambiguous_build_dir(Path::new(
            "src/main/java/com/example/Foo.java"
        )));
        assert!(!is_in_ambiguous_build_dir(Path::new("Foo.java")));
        assert!(!is_in_ambiguous_build_dir(Path::new(
            "app/src/main/java/Foo.java"
        )));
    }
}
