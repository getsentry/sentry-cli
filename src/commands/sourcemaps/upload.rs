#![expect(clippy::unwrap_used, reason = "contains legacy code which uses unwrap")]

use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use glob::{glob_with, MatchOptions};
use itertools::Itertools as _;
use log::{debug, warn};

use crate::api::{Api, ChunkUploadCapability};
use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::validate_distribution;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{SourceFile, UploadContext};
use crate::utils::fs::path_as_url;
use crate::utils::sourcemaps::SourceMapProcessor;

const DEFAULT_EXTENSIONS: &[&str] = &["js", "cjs", "mjs", "map", "jsbundle", "bundle"];

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload sourcemaps for a release.")
        // Backward compatibility with `releases files <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
        .arg(
            Arg::new("paths")
                .value_name("PATHS")
                .required_unless_present_any(["bundle", "bundle_sourcemap"])
                .num_args(1..)
                .action(ArgAction::Append)
                .help("The files to upload."),
        )
        .arg(
            Arg::new("url_prefix")
                .short('u')
                .long("url-prefix")
                .value_name("PREFIX")
                .help("The URL prefix to prepend to all filenames."),
        )
        .arg(
            Arg::new("url_suffix")
                .long("url-suffix")
                .value_name("SUFFIX")
                .help("The URL suffix to append to all filenames."),
        )
        .arg(
            Arg::new("dist")
                .long("dist")
                .short('d')
                .value_name("DISTRIBUTION")
                .value_parser(validate_distribution)
                .help("Optional distribution identifier for the sourcemaps."),
        )
        .arg(
            Arg::new("note")
                .long("note")
                .value_name("NOTE")
                .help("Adds an optional note to the uploaded artifact bundle."),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .action(ArgAction::SetTrue)
                .help("Enable basic sourcemap validation."),
        )
        .arg(
            Arg::new("decompress")
                .long("decompress")
                .action(ArgAction::SetTrue)
                .help("Enable files gzip decompression prior to upload."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .conflicts_with("wait_for")
                .help("Wait for the server to fully process uploaded files."),
        )
        .arg(
            Arg::new("wait_for")
                .long("wait-for")
                .value_name("SECS")
                .value_parser(clap::value_parser!(u64))
                .conflicts_with("wait")
                .help(
                    "Wait for the server to fully process uploaded files, \
                     but at most for the given number of seconds.",
                ),
        )
        .arg(
            Arg::new("no_sourcemap_reference")
                .long("no-sourcemap-reference")
                .action(ArgAction::SetTrue)
                .help(
                    "Disable emitting of automatic sourcemap references.{n}\
                    By default the tool will store a 'Sourcemap' header with \
                    minified files so that sourcemaps are located automatically \
                    if the tool can detect a link. If this causes issues it can \
                    be disabled.",
                ),
        )
        .arg(
            Arg::new("debug_id_reference")
                .long("debug-id-reference")
                .action(ArgAction::SetTrue)
                .help(
                    "Enable emitting of automatic debug id references.{n}\
                  By default Debug ID reference has to be present both \
                  in the source and the related sourcemap. But in cases \
                  of binary bundles, the tool can't verify presence of \
                  the Debug ID. This flag allows use of Debug ID from \
                  the linked sourcemap.",
                ),
        )
        .arg(
            Arg::new("no_rewrite")
                .long("no-rewrite")
                .action(ArgAction::SetTrue)
                .help(
                    "Disables rewriting of matching sourcemaps. By default the tool \
                    will rewrite sources, so that indexed maps are flattened and missing \
                    sources are inlined if possible.{n}This fundamentally \
                    changes the upload process to be based on sourcemaps \
                    and minified files exclusively and comes in handy for \
                    setups like react-native that generate sourcemaps that \
                    would otherwise not work for sentry.",
                ),
        )
        .arg(
            Arg::new("strip_prefix")
                .long("strip-prefix")
                .value_name("PREFIX")
                .action(ArgAction::Append)
                .help(
                    "Strips the given prefix from all sources references inside the upload \
                    sourcemaps (paths used within the sourcemap content, to map minified code \
                    to it's original source). Only sources that start with the given prefix \
                    will be stripped.{n}This will not modify the uploaded sources paths. \
                    To do that, point the upload or upload-sourcemaps command \
                    to a more precise directory instead.",
                )
                .conflicts_with("no_rewrite"),
        )
        .arg(
            Arg::new("strip_common_prefix")
                .long("strip-common-prefix")
                .action(ArgAction::SetTrue)
                .help(
                    "Similar to --strip-prefix but strips the most common \
                    prefix on all sources references.",
                )
                .conflicts_with("no_rewrite"),
        )
        .arg(
            Arg::new("ignore")
                .long("ignore")
                .short('i')
                .value_name("IGNORE")
                .action(ArgAction::Append)
                .help("Ignores all files and folders matching the given glob"),
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
            Arg::new("bundle")
                .long("bundle")
                .value_name("BUNDLE")
                .conflicts_with("paths")
                .requires("bundle_sourcemap")
                .help("Path to the application bundle (indexed, file, or regular)"),
        )
        .arg(
            Arg::new("bundle_sourcemap")
                .long("bundle-sourcemap")
                .value_name("BUNDLE_SOURCEMAP")
                .conflicts_with("paths")
                .requires("bundle")
                .help("Path to the bundle sourcemap"),
        )
        .arg(
            Arg::new("no_dedupe")
                .long("no-dedupe")
                .action(ArgAction::SetTrue)
                .help(
                    "Skip artifacts deduplication prior to uploading. \
                    This will force all artifacts to be uploaded, \
                    no matter whether they are already present on the server.",
                ),
        )
        .arg(
            Arg::new("extensions")
                .long("ext")
                .short('x')
                .value_name("EXT")
                .action(ArgAction::Append)
                .help(format!(
                    "Set the file extensions that are considered for upload. \
                    This overrides the default extensions. To add an extension, all default \
                    extensions must be repeated. Specify once per extension.\n\
                    Defaults to: `{}`",
                    DEFAULT_EXTENSIONS
                        .iter()
                        .map(|ext| format!("--ext={ext}"))
                        .join(" ")
                )),
        )
        .arg(
            Arg::new("strict")
                .long("strict")
                .short('s')
                .action(ArgAction::SetTrue)
                .help(
                    "Fail with a non-zero exit code if the specified source map file cannot be \
                     uploaded.",
                ),
        )
        .arg(
            Arg::new("use_artifact_bundle")
                .long("use-artifact-bundle")
                .action(ArgAction::SetTrue)
                .help(
                    "[DEPRECATED] Force artifact bundles to be used for upload, even when not \
                    supported by the server. This option has always only been intended for \
                    internal use, and it is now officially deprecated.",
                )
                .hide(true),
        )
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(
            Arg::new("rewrite")
                .long("rewrite")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .action(ArgAction::SetTrue)
                .short('v')
                .hide(true),
        )
}

fn get_prefixes_from_args(matches: &ArgMatches) -> Vec<&str> {
    let mut prefixes: Vec<&str> = match matches.get_many::<String>("strip_prefix") {
        Some(paths) => paths.map(String::as_str).collect(),
        None => vec![],
    };
    if matches.get_flag("strip_common_prefix") {
        prefixes.push("~");
    }
    prefixes
}

fn process_sources_from_bundle(
    matches: &ArgMatches,
    processor: &mut SourceMapProcessor,
) -> Result<()> {
    let url_suffix = matches
        .get_one::<String>("url_suffix")
        .map(String::as_str)
        .unwrap_or_default();
    let mut url_prefix = matches
        .get_one::<String>("url_prefix")
        .map(String::as_str)
        .unwrap_or("~");
    // remove a single slash from the end.  so ~/ becomes ~ and app:/// becomes app://
    if url_prefix.ends_with('/') {
        url_prefix = &url_prefix[..url_prefix.len() - 1];
    }

    let bundle_path = PathBuf::from(matches.get_one::<String>("bundle").unwrap());
    let bundle_url = format!(
        "{url_prefix}/{}{url_suffix}",
        bundle_path.file_name().unwrap().to_string_lossy()
    );

    let sourcemap_path = PathBuf::from(matches.get_one::<String>("bundle_sourcemap").unwrap());
    let sourcemap_url = format!(
        "{url_prefix}/{}{url_suffix}",
        sourcemap_path.file_name().unwrap().to_string_lossy()
    );

    debug!("Bundle path: {}", bundle_path.display());
    debug!("Sourcemap path: {}", sourcemap_path.display());

    processor.add(
        &bundle_url,
        ReleaseFileSearch::collect_file(bundle_path.clone())?,
    );
    let sourcemap_match = ReleaseFileSearch::collect_file(sourcemap_path)?;

    if let Ok(ram_bundle) = sourcemap::ram_bundle::RamBundle::parse_unbundle_from_path(&bundle_path)
    {
        debug!("File RAM bundle found, extracting its contents...");
        // For file ("unbundle") RAM bundles we need to explicitly unpack it, otherwise we cannot detect it
        // reliably inside "processor.rewrite()"

        let sourcemap_source = SourceFile::from_release_file_match(&sourcemap_url, sourcemap_match);
        processor.unpack_ram_bundle(&ram_bundle, &bundle_url, &sourcemap_source)?;
    } else if sourcemap::ram_bundle::RamBundle::parse_indexed_from_path(&bundle_path).is_ok() {
        debug!("Indexed RAM bundle found");
        let sourcemap_source = SourceFile::from_release_file_match(&sourcemap_url, sourcemap_match);
        processor.unpack_indexed_ram_bundles(&sourcemap_source)?;
    } else {
        warn!("Regular bundle found");
        processor.add(&sourcemap_url, sourcemap_match);
    }

    let mut prefixes = get_prefixes_from_args(matches);
    if !prefixes.contains(&"~") {
        prefixes.push("~");
    }
    debug!("Prefixes: {:?}", prefixes);

    processor.rewrite(&prefixes)?;
    processor.add_sourcemap_references();
    processor.add_debug_id_references();

    Ok(())
}

fn process_sources_from_paths(
    matches: &ArgMatches,
    processor: &mut SourceMapProcessor,
) -> Result<()> {
    let paths = matches.get_many::<String>("paths").unwrap();
    let ignore_file = matches
        .get_one::<String>("ignore_file")
        .map(String::as_str)
        .unwrap_or_default();
    let extensions = matches
        .get_many::<String>("extensions")
        .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
        .unwrap_or_else(|| DEFAULT_EXTENSIONS.to_vec());
    let ignores: Vec<_> = matches
        .get_many::<String>("ignore")
        .map(|ignores| ignores.map(|i| format!("!{i}")).collect())
        .unwrap_or_default();

    let opts = MatchOptions::new();
    let collected_paths = paths.flat_map(|path| glob_with(path, opts).unwrap().flatten());

    for path in collected_paths {
        // if we start walking over something that is an actual file then
        // the directory iterator yields that path and terminates.  We
        // handle that case here specifically to figure out what the path is
        // we should strip off.
        let path = path.as_path();
        let (base_path, check_ignore) = if path.is_file() {
            (path.parent().unwrap(), false)
        } else {
            (path, true)
        };

        let mut search = ReleaseFileSearch::new(path.to_path_buf());
        search.decompress(matches.get_flag("decompress"));

        if check_ignore {
            search
                .ignore_file(ignore_file)
                .ignores(ignores.clone())
                .extensions(extensions.clone());
        }

        let sources = search.collect_files()?;

        let url_suffix = matches
            .get_one::<String>("url_suffix")
            .map(String::as_str)
            .unwrap_or_default();
        let mut url_prefix = matches
            .get_one::<String>("url_prefix")
            .map(String::as_str)
            .unwrap_or("~");
        // remove a single slash from the end.  so ~/ becomes ~ and app:/// becomes app://
        if url_prefix.ends_with('/') {
            url_prefix = &url_prefix[..url_prefix.len() - 1];
        }

        for source in sources {
            let local_path = source.path.strip_prefix(base_path).unwrap();
            let url = format!("{url_prefix}/{}{url_suffix}", path_as_url(local_path));
            processor.add(&url, source);
        }
    }

    if !matches.get_flag("no_rewrite") {
        let prefixes = get_prefixes_from_args(matches);
        processor.rewrite(&prefixes)?;
    }

    if !matches.get_flag("no_sourcemap_reference") {
        processor.add_sourcemap_references();
    }

    if matches.get_flag("debug_id_reference") {
        processor.add_debug_id_references();
    }

    if matches.get_flag("validate") {
        processor.validate_all()?;
    }

    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let version = config.get_release_with_legacy_fallback(matches).ok();
    let org = config.get_org(matches)?;
    let projects = config.get_projects(matches)?;
    let api = Api::current();
    let mut processor = SourceMapProcessor::new();
    let mut chunk_upload_options = api.authenticated()?.get_chunk_upload_options(&org)?;

    if matches.get_flag("use_artifact_bundle")
        || env::var("SENTRY_FORCE_ARTIFACT_BUNDLES").ok().as_deref() == Some("1")
    {
        log::warn!("The --use-artifact-bundle option and the SENTRY_FORCE_ARTIFACT_BUNDLES environment variable \
                    are both deprecated, and both will be removed in the next major version.");

        if let Some(ref mut options) = chunk_upload_options {
            if !options.supports(ChunkUploadCapability::ArtifactBundles) {
                options.accept.push(ChunkUploadCapability::ArtifactBundles);
            }
        }
    }

    if matches.contains_id("bundle") && matches.contains_id("bundle_sourcemap") {
        process_sources_from_bundle(matches, &mut processor)?;
    } else {
        process_sources_from_paths(matches, &mut processor)?;
    }

    let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
    let wait = matches.get_flag("wait") || wait_for_secs.is_some();
    let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, Duration::from_secs);
    let upload_context = UploadContext {
        org: &org,
        projects: &projects,
        release: version.as_deref(),
        dist: matches.get_one::<String>("dist").map(String::as_str),
        note: matches.get_one::<String>("note").map(String::as_str),
        wait,
        max_wait,
        dedupe: !matches.get_flag("no_dedupe"),
        chunk_upload_options: chunk_upload_options.as_ref(),
    };

    if matches.get_flag("strict") {
        processor.upload_strict(&upload_context)?;
    } else {
        processor.upload(&upload_context)?;
    }

    Ok(())
}
