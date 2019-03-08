use std::io;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use console::style;
use failure::Error;

use crate::utils::dif::DifFile;
use crate::utils::system::QuietExit;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Check the debug info file at a given path.")
        .arg(
            Arg::with_name("type")
                .long("type")
                .short("t")
                .value_name("TYPE")
                .possible_values(&["dsym", "elf", "proguard", "breakpad"])
                .help(
                    "Explicitly set the type of the debug info file. \
                     This should not be needed as files are auto detected.",
                ),
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .help("Format outputs as JSON."),
        )
        .arg(
            Arg::with_name("path")
                .index(1)
                .required(true)
                .help("The path to the debug info file."),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let dif = DifFile::open_path(path, ty)?;

    if matches.is_present("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &dif)?;
        println!();
        return if dif.is_usable() {
            Ok(())
        } else {
            Err(QuietExit(1).into())
        };
    }

    println!("{}", style("Debug Info File Check").dim().bold());
    match dif.kind() {
        Some(class) => println!(
            "  Type: {} {:#}",
            style(dif.ty()).cyan(),
            style(class).cyan()
        ),
        None => println!("  Type: {}", style(dif.ty()).cyan()),
    }

    println!("  Contained debug identifiers:");
    for (id, cpu_type) in dif.variants() {
        match cpu_type {
            Some(cpu_type) => println!("    > {} ({})", style(id).dim(), style(cpu_type).cyan()),
            None => println!("    > {}", style(id).dim()),
        }
    }

    println!("  Contained debug information:");
    println!("    > {}", dif.features());

    if let Some(msg) = dif.get_note() {
        println!("  Note: {}", msg);
    }

    if let Some(prob) = dif.get_problem() {
        println!("  Usable: {} ({})", style("no").red(), prob);
        Err(QuietExit(1).into())
    } else {
        println!("  Usable: {}", style("yes").green());
        Ok(())
    }
}
