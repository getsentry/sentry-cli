use std::io;
use std::path::Path;

use clap::{App, ArgMatches};
use failure::Error;
use serde_json;

use utils::dif::DifFile;
use utils::system::QuietExit;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Print debug identifier(s) from a debug info file.")
        (@arg types: -t --type [TYPE]... possible_values(&["dsym", "proguard", "breakpad"])
            "Explicitly set the type of the debug info file. \
             This should not be needed as files are auto detected.")
        (@arg json: --json "Format outputs as JSON.")
        (@arg path: <PATH> "The path to the debug info file.")
    )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let f = DifFile::open_path(path, ty)?;

    if !f.is_usable() {
        println_stderr!(
            "error: debug info file is not usable: {}",
            f.get_problem().unwrap_or("unknown error")
        );
        return Err(QuietExit(1).into());
    }

    if !matches.is_present("json") {
        for id in f.ids() {
            println!("{}", id);
        }
    } else if matches.is_present("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &f.ids())?;
        println!();
    }

    Ok(())
}
