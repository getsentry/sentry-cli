use std::fs;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use failure::Error;
use log::warn;
use symbolic::debuginfo::sourcebundle::SourceBundleWriter;

use crate::utils::dif::DifFile;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Create a source bundle for a given debug information file")
        .arg(
            Arg::with_name("paths")
                .index(1)
                .required(true)
                .multiple(true)
                .help("The path to the input debug info files."),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("PATH")
                .help(
                    "The path to the output folder.  If not provided the \
                     file is placed next to the input file.",
                ),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let output_path = matches.value_of("output").map(Path::new);

    for orig_path in matches.values_of("paths").unwrap() {
        let path = Path::new(orig_path).canonicalize()?;
        let archive = match DifFile::open_path(&path, None)? {
            DifFile::Archive(archive) => archive,
            _ => {
                warn!("Cannot build source bundles from {} files", orig_path);
                continue;
            }
        };
        let archive = archive.get();

        for (idx, object) in archive.objects().enumerate() {
            let object = object?;
            if object.has_sources() {
                println!("skipping {} because it contains source info", orig_path);
                continue;
            }

            let mut out = output_path
                .unwrap_or_else(|| path.parent().unwrap())
                .join(path.file_name().unwrap());
            if idx > 1 {
                out.set_extension(&format!("{}.src.zip", idx));
            } else {
                out.set_extension("src.zip");
            }
            fs::create_dir_all(out.parent().unwrap())?;
            let writer = SourceBundleWriter::create(&out)?;

            // Resolve source files from the object and write their contents into the archive. Skip to
            // upload this bundle if no source could be written. This can happen if there is no file or
            // line information in the object file, or if none of the files could be resolved.
            let written =
                writer.write_object(&object, &path.file_name().unwrap().to_string_lossy())?;
            if !written {
                warn!("Could not find any sources for {}", orig_path);
                fs::remove_file(&out)?;
                continue;
            } else {
                println!("Written sources for {} to {}", orig_path, out.display());
            }
        }
    }

    Ok(())
}
