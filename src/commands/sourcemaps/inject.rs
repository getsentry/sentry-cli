use std::path::PathBuf;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::fs::path_as_url;
use crate::utils::sourcemaps::SourceMapProcessor;

pub fn make_command(command: Command) -> Command {
    command
        .about("Fixes up JavaScript source files and sourcemaps with debug ids.")
        .long_about(
            "Fixes up JavaScript source files and sourcemaps with debug ids.{n}{n}\
            For every minified JS source file, a debug id is generated and \
            inserted into the file. If the source file references a \
            sourcemap and that sourcemap is locally available, \
            the debug id will be injected into it as well. \
            If the referenced sourcemap already contains a debug id, \
            that id is used instead.",
        )
        .arg(
            Arg::new("paths")
                .value_name("PATHS")
                .num_args(1..)
                .required(true)
                .action(ArgAction::Append)
                .help(
                    "A path to recursively search for javascript files that should be processed.",
                ),
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
            Arg::new("extensions")
                .long("ext")
                .short('x')
                .value_name("EXT")
                .action(ArgAction::Append)
                .help(
                    "Set the file extensions of JavaScript files that are considered \
                    for injection.  This overrides the default extensions (js, cjs, mjs).  \
                    To add an extension, all default extensions must be repeated.  Specify \
                    once per extension.  Source maps are discovered via those files.",
                ),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Don't modify files on disk."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let mut processor = SourceMapProcessor::new();

    #[expect(clippy::unwrap_used, reason = "legacy code")]
    let paths = matches
        .get_many::<String>("paths")
        .unwrap()
        .map(PathBuf::from);
    let dry_run = matches.get_flag("dry_run");

    let ignore_file = matches
        .get_one::<String>("ignore_file")
        .map(String::as_str)
        .unwrap_or_default();
    let ignores: Vec<_> = matches
        .get_many::<String>("ignore")
        .map(|ignores| ignores.map(|i| format!("!{i}")).collect())
        .unwrap_or_default();

    let mut extensions = matches
        .get_many::<String>("extensions")
        .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
        .unwrap_or_else(|| vec!["js", "cjs", "mjs"]);

    // Sourcemaps should be discovered regardless of which JavaScript extensions have been selected.
    extensions.push("map");

    for path in paths {
        println!("> Searching {}", path.display());
        let sources = ReleaseFileSearch::new(path)
            .ignore_file(ignore_file)
            .ignores(&ignores)
            .extensions(extensions.clone())
            .collect_files()?;
        for source in sources {
            let url = path_as_url(&source.path);
            processor.add(&url, source);
        }
    }

    processor.inject_debug_ids(dry_run, &extensions)?;
    Ok(())
}
