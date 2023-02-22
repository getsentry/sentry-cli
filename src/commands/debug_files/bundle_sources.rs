use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use log::warn;
use symbolic::debuginfo::sourcebundle::SourceBundleWriter;

use crate::utils::dif::DifFile;
use crate::utils::dif_upload::filter_bad_sources;

pub fn make_command(command: Command) -> Command {
    command
        .about("Create a source bundle for a given debug information file")
        .arg(
            Arg::new("paths")
                .required(true)
                .multiple_values(true)
                .action(ArgAction::Append)
                .help("The path to the input debug info files."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("PATH")
                .help(
                    "The path to the output folder.  If not provided the \
                     file is placed next to the input file.",
                ),
        )
}

fn is_dsym(path: &Path) -> bool {
    path.extension().map_or(false, |e| e == "dSYM")
}

fn get_sane_parent(path: &Path) -> &Path {
    let mut parent = path.parent().unwrap();

    if parent.ends_with("Contents/Resources/DWARF") {
        let mut dsym_parent = parent;
        for _ in 0..3 {
            dsym_parent = dsym_parent.parent().unwrap();
        }
        if is_dsym(dsym_parent) {
            parent = dsym_parent.parent().unwrap();
        }
    }

    parent
}

fn get_canonical_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let mut canonical_path = Path::new(path.as_ref()).canonicalize()?;

    if is_dsym(&canonical_path) {
        if let Some(dsym_name) = canonical_path.file_stem() {
            let mut dsym_path = canonical_path.join("Contents/Resources/DWARF");
            dsym_path.push(dsym_name);
            if dsym_path.is_file() {
                canonical_path = dsym_path;
            }
        }
    }

    Ok(canonical_path)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let output_path = matches.get_one::<String>("output").map(Path::new);

    for orig_path in matches.get_many::<String>("paths").unwrap() {
        let canonical_path = get_canonical_path(orig_path)?;

        let archive = match DifFile::open_path(&canonical_path, None)? {
            DifFile::Archive(archive) => archive,
            _ => {
                warn!("Cannot build source bundles from {}", orig_path);
                continue;
            }
        };

        // At this point we can be sure that we're dealing with a file
        let parent_path = get_sane_parent(&canonical_path);
        let filename = canonical_path.file_name().unwrap();

        for (index, object) in archive.get().objects().enumerate() {
            let object = object?;
            if object.has_sources() {
                eprintln!("skipped {orig_path} (no source info)");
                continue;
            }

            let mut out = output_path.unwrap_or(parent_path).join(filename);
            match index {
                0 => out.set_extension("src.zip"),
                index => out.set_extension(&format!("{index}.src.zip")),
            };

            fs::create_dir_all(out.parent().unwrap())?;
            let writer = SourceBundleWriter::create(&out)?;

            // Resolve source files from the object and write their contents into the archive. Skip to
            // upload this bundle if no source could be written. This can happen if there is no file or
            // line information in the object file, or if none of the files could be resolved.
            let written = writer.write_object_with_filter(
                &object,
                &filename.to_string_lossy(),
                |file, _source_descriptor| filter_bad_sources(file),
            )?;

            if !written {
                eprintln!("skipped {orig_path} (no files found)");
                fs::remove_file(&out)?;
                continue;
            } else {
                println!("{}", out.display());
            }
        }
    }

    Ok(())
}
