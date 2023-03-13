use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use log::{debug, warn};
use walkdir::WalkDir;

use crate::utils::sourcemaps::inject::{inject_file, InjectReport};

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
        .hide(true)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = matches.get_one::<String>("path").unwrap();

    let mut collected_paths = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(ref e) => {
                debug!("Skipping file: {e}");
                continue;
            }
        };

        if entry
            .path()
            .extension()
            .map_or(false, |ext| ext == "js" || ext == "cjs" || ext == "mjs")
        {
            collected_paths.push(entry.path().to_owned());
        }
    }

    if collected_paths.is_empty() {
        warn!("Did not find any JavaScript files in path: {path}",);
        return Ok(());
    }

    let mut report = InjectReport::default();
    for path in &collected_paths {
        inject_file(path, &mut report)?;
    }

    println!("{report}");

    Ok(())
}
