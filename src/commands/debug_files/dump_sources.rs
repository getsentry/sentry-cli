use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use symbolic::common::ByteView;
use symbolic::debuginfo::{Archive, Object};

pub fn make_command(command: Command) -> Command {
    command
        .about("Print source files linked by the given debug info file.")
        .arg(
            Arg::new("path")
                .required(true)
                .help("The path to the debug info file."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());

    // which types should we consider?
    let data = ByteView::open(path)?;
    let archive = Archive::parse(&data)?;

    if archive.object_count() == 0 {
        println!("No objects found in the given debug info file");
        return Ok(());
    }

    for object in archive.objects() {
        let object = object?;

        print_object_sources(&object)?;

        // In case of a PE file with an embedded PDB, handle the PPDB separately.
        if let Object::Pe(pe) = &object {
            if let Some(ppdb_data) = pe.embedded_ppdb()? {
                let mut buf = Vec::new();
                ppdb_data.decompress_to(&mut buf)?;
                let ppdb = Object::parse(&buf)?;
                print_object_sources(&ppdb)?;
            }
        }
    }

    Ok(())
}

fn print_object_sources(object: &Object) -> Result<()> {
    if !object.has_sources() {
        println!(
            "{} {} has no sources.",
            object.file_format(),
            object.debug_id()
        );
    } else {
        println!(
            "{} {} references sources:",
            object.file_format(),
            object.debug_id()
        );
        let debug_session = object.debug_session()?;
        for file in debug_session.files() {
            let file = file?;
            println!("  {}", file.path_str());
            let abs_path = file.abs_path_str();
            match debug_session.source_by_path(abs_path.as_str())? {
                Some(source) => {
                    println!("    Embedded, {} bytes", source.len());
                }
                None => {
                    if Path::new(&abs_path).exists() {
                        println!("    Not embedded, but available on the local disk.");
                    } else {
                        println!("    Not available in file or on disk.");
                    }
                }
            }
        }
    }
    Ok(())
}
