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
    let path = Path::new(matches.get_one::<String>("path").unwrap());

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
    let debug_session = object.debug_session()?;

    // We're not using object.has_sources() because it only reports on embedded sources, not referenced files.
    if debug_session.files().next().is_none() {
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
        for file in debug_session.files() {
            let file = file?;
            let abs_path = file.abs_path_str();
            println!("  {}", &abs_path);
            let source = debug_session.source_by_path(abs_path.as_str())?;
            match source.as_ref().and_then(|sd| sd.contents()) {
                Some(source) => {
                    println!("    Embedded, {} bytes", source.len());
                }
                None => {
                    if Path::new(&abs_path).exists() {
                        println!("    Not embedded, but available on the local disk.");
                    } else {
                        println!("    Not embedded nor available locally at the referenced path.");
                    }
                }
            }
        }
    }
    Ok(())
}
