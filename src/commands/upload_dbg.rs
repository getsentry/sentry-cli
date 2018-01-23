use std::collections::BTreeSet;
use std::path::PathBuf;

use clap::{App, AppSettings, Arg, ArgMatches};
use console::style;
use symbolic_common::{ObjectClass, ObjectKind};
use uuid::Uuid;

use prelude::*;
use utils::{validate_uuid, ArgExt};
use utils::upload::{process_batch, BatchedObjectWalker, UploadOptions};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload Linux debug symbols to a project.")
        .setting(AppSettings::Hidden)
        .org_project_args()
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("A path to search recursively for symbol files.")
            .multiple(true)
            .number_of_values(1)
            .index(1))
        .arg(Arg::with_name("no_executables")
            .long("no-executables")
            .help("Exclude executables and look for stripped symbols only."))
        .arg(Arg::with_name("no_debug_only")
            .long("no-debug-only")
            .help("Exclude files only containing stripped debugging infos.")
            .conflicts_with("no_executables"))
        .arg(Arg::with_name("uuids")
            .value_name("UUID")
            .long("uuid")
            .help("Search for specific UUIDs.")
            .validator(validate_uuid)
            .multiple(true)
            .number_of_values(1))
        .arg(Arg::with_name("require_all")
            .long("require-all")
            .help("Errors if not all UUIDs specified with --uuid could be found."))
        .arg(Arg::with_name("no_reprocessing")
            .long("no-reprocessing")
            .help("Do not trigger reprocessing after uploading."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let exes = !matches.is_present("no_executables");
    let dbgs = !matches.is_present("no_debug_only");
    let paths = match matches.values_of("paths") {
        Some(paths) => paths.map(|path| PathBuf::from(path)).collect(),
        None => vec![],
    };

    if paths.len() == 0 {
        // We allow this because reprocessing will still be triggered
        println!("Warning: no paths were provided.");
    }

    let mut found = BTreeSet::new();
    let uuids = matches.values_of("uuids").map_or(BTreeSet::new(), |uuids| {
        uuids.map(|s| Uuid::parse_str(s).unwrap()).collect()
    });

    let context = UploadOptions::from_cli(matches)?;
    let mut total_uploaded = 0;

    // Search all paths and upload symbols in batches
    for path in paths.into_iter() {
        let mut iter = BatchedObjectWalker::new(path, &mut found);
        iter.object_kind(ObjectKind::Elf)
            .object_uuids(uuids.clone())
            .max_batch_size(context.max_size());

        if exes {
            iter.object_class(ObjectClass::Executable)
                .object_class(ObjectClass::Library);
        }

        if dbgs {
            iter.object_class(ObjectClass::Debug);
        }

        for (i, batch) in iter.enumerate() {
            if i > 0 {
                println!("");
            }

            println!("{}", style(format!("Batch {}", i)).bold());
            total_uploaded += process_batch(batch?, &context)?;
        }
    }

    if total_uploaded > 0 {
        println!("Uploaded a total of {} symbols", style(total_uploaded).yellow());
    }

    // Trigger reprocessing only if requested by user
    if matches.is_present("no_reprocessing") {
        println!("{} skipped reprocessing", style(">").dim());
    } else if !context.api().trigger_reprocessing(&context.org(), &context.project())? {
        println!("{} Server does not support reprocessing. Not triggering.", style(">").dim());
    }

    // Did we miss explicitly requested symbols?
    if matches.is_present("require_all") && !uuids.is_empty() {
        let missing: BTreeSet<_> = uuids.difference(&found).collect();
        if !missing.is_empty() {
            println!("");

            println_stderr!("{}", style("error: not all requested dsyms could be found.").red());
            println_stderr!("The following symbols are still missing:");
            for uuid in &missing {
                println!("  {}", uuid);
            }

            return Err(ErrorKind::QuietExit(1).into());
        }
    }

    Ok(())
}
