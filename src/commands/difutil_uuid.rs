use std::io;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use serde_json;

use prelude::*;
use utils::dif::DifFile;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app
        .about("Print UUID(s) from a debug info file.")
        .arg(Arg::with_name("type")
             .long("type")
             .short("t")
             .value_name("TYPE")
             .possible_values(&["dsym", "proguard", "breakpad"])
             .help("Explicitly set the type of the debug info file. \
                    This should not be needed as files are auto detected."))
        .arg(Arg::with_name("json")
             .long("json")
             .help("Format outputs as JSON."))
        .arg(Arg::with_name("path")
             .index(1)
             .required(true)
             .help("The path to the debug info file."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let f = DifFile::open_path(path, ty)?;

    if !f.is_usable() {
        println_stderr!("error: debug info file is not usable: {}",
                        f.get_problem().unwrap_or("unknown error"));
        return Err(ErrorKind::QuietExit(1).into());
    }

    if !matches.is_present("json") {
        for uuid in f.uuids() {
            println!("{}", uuid);
        }
    } else {
        if matches.is_present("json") {
            serde_json::to_writer_pretty(&mut io::stdout(), &f.uuids())?;
            println!("");
        }
    }

    Ok(())
}
