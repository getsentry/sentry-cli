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
            For every JS source file that references a sourcemap, a debug id is generated and \
            inserted into both files. If the referenced sourcemap already contains a debug id, \
            that id is used instead.",
        )
        .arg(
            Arg::new("paths")
                .value_name("PATHS")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("A path to recursively search for javascript files that should be processed."),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Don't modify files on disk."),
        )
        .hide(true)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let mut processor = SourceMapProcessor::new();

    let paths = matches
        .get_many::<String>("paths")
        .unwrap()
        .map(PathBuf::from);
    let dry_run = matches.get_flag("dry_run");

    for path in paths {
        let search = ReleaseFileSearch::new(path);
        let sources = search.collect_files()?;
        for source in sources {
            let url = path_as_url(&source.path);
            processor.add(&url, source)?;
        }
    }

    processor.inject_debug_ids(dry_run)?;
    Ok(())
}
