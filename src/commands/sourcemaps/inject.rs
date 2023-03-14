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
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path to the javascript files."),
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
    let path: PathBuf = matches.get_one::<String>("path").unwrap().into();
    let dry_run = matches.get_flag("dry_run");

    let search = ReleaseFileSearch::new(path);
    let sources = search.collect_files()?;
    for source in sources {
        let url = path_as_url(&source.path);
        processor.add(&url, source)?;
    }

    processor.inject_debug_ids(dry_run)?;
    Ok(())
}
