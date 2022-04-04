use std::io;
use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::style;

use crate::utils::dif::DifFile;
use crate::utils::logging::is_quiet_mode;
use crate::utils::system::QuietExit;

pub fn make_command(command: Command) -> Command {
    command
        .about("Check the debug info file at a given path.")
        .arg(
            Arg::new("type")
                .long("type")
                .short('t')
                .value_name("TYPE")
                .possible_values(&["dsym", "elf", "proguard", "breakpad"])
                .help(
                    "Explicitly set the type of the debug info file. \
                     This should not be needed as files are auto detected.",
                ),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Format outputs as JSON."),
        )
        .arg(
            Arg::new("path")
                .index(1)
                .required(true)
                .help("The path to the debug info file."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let ty = matches.value_of("type").map(|t| t.parse().unwrap());
    let dif = DifFile::open_path(path, ty)?;

    if matches.is_present("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &dif)?;
        println!();
    }

    if matches.is_present("json") || is_quiet_mode() {
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
    for variant in dif.variants() {
        println!("    > Debug ID: {}", style(variant.debug_id).dim());
        if let Some(code_id) = variant.code_id {
            println!("      Code ID:  {}", style(code_id).dim());
        }
        if let Some(arch) = variant.arch {
            println!("      Arch:     {}", style(arch).dim());
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
