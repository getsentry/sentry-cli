use std::io;
use std::path::Path;

use clap::{App, ArgMatches};
use console::style;
use failure::Error;
use serde_json;

use utils::dif::DifFile;
use utils::system::QuietExit;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Check the debug info file at a given path.")
        (@arg type: -t --type [TYPE] possible_values(&["dsym", "proguard", "breakpad"])
            "Explicitly set the type of the debug info file. \
             This should not be needed as files are auto detected.")
        (@arg json: --json "Format outputs as JSON.")
        (@arg path: +required "The path to the debug info file.")
    )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let f = DifFile::open_path(path, ty)?;

    if matches.is_present("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &f)?;
        println!();
        return if f.is_usable() {
            Ok(())
        } else {
            Err(QuietExit(1).into())
        };
    }

    println!("{}", style("Debug Info File Check").dim().bold());
    println!("  Type: {}", style(f.ty()).cyan());
    println!("  Contained debug identifiers:");
    for (id, cpu_type) in f.variants() {
        if let Some(cpu_type) = cpu_type {
            println!("    > {} ({})", style(id).dim(), style(cpu_type).cyan());
        } else {
            println!("    > {}", style(id).dim());
        }
    }

    if let Some(msg) = f.get_note() {
        println!("  Note: {}", msg);
    }

    if let Some(prob) = f.get_problem() {
        println!("  Usable: {} ({})", style("no").red(), prob);
        Err(QuietExit(1).into())
    } else {
        println!("  Usable: {}", style("yes").green());
        Ok(())
    }
}
