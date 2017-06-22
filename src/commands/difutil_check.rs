use std::io;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use console::style;
use serde_json;

use prelude::*;
use config::Config;
use utils::dif;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app
        .about("given the path to a debug info file it checks it")
        .arg(Arg::with_name("type")
             .long("type")
             .short("t")
             .value_name("TYPE")
             .possible_values(&["dsym", "proguard"])
             .help("Explicitly sets the type of the debug info file."))
        .arg(Arg::with_name("json")
             .long("json")
             .help("Returns the results as JSON"))
        .arg(Arg::with_name("path")
             .index(1)
             .required(true)
             .help("The path to the debug info file."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let f = dif::DifFile::open_path(path, ty)?;

    if matches.is_present("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &f)?;
        println!("");
        return if f.is_usable() {
            Ok(())
        } else {
            Err(ErrorKind::QuietExit(1).into())
        };
    }

    println!("{}", style("Debug Info File Check").dim().bold());
    println!("  Type: {}", style(f.ty()).cyan());
    println!("  Contained UUIDs:");
    for (uuid, cpu_type) in f.variants() {
        if let Some(cpu_type) = cpu_type {
            println!("    > {} ({})", style(uuid).dim(), style(cpu_type).cyan());
        } else {
            println!("    > {}", style(uuid).dim());
        }
    }

    if let Some(prob) = f.get_problem() {
        println!("  Usable: {} ({})", style("no").red(), prob);
        Err(ErrorKind::QuietExit(1).into())
    } else {
        println!("  Usable: {}", style("yes").green());
        Ok(())
    }
}
