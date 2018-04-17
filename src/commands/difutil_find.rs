use std::io;
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::collections::HashSet;

use clap::{App, Arg, ArgMatches};
use console::style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde_json;
use symbolic_common::ByteView;
use symbolic_debuginfo::DebugId;
use symbolic_proguard::ProguardMappingView;
use uuid::UuidVersion;
use walkdir::WalkDir;
use failure::Error;

use utils::system::QuietExit;
use utils::args::validate_id;
use utils::dif::{DifFile, DifType};

// text files larger than 32 megabytes are not considered to be
// valid mapping files when scanning
const MAX_MAPPING_FILE: u64 = 32 * 1024 * 1024;

#[derive(Serialize, Debug)]
struct DifMatch {
    #[serde(rename = "type")]
    pub ty: DifType,
    pub id: DebugId,
    pub path: PathBuf,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Locate debug information files for given debug identifiers.")
        .arg(
            Arg::with_name("types")
                .long("type")
                .short("t")
                .value_name("TYPE")
                .multiple(true)
                .number_of_values(1)
                .possible_values(&["dsym", "proguard", "breakpad"])
                .help(
                    "Only consider debug information files of the given \
                     type.  By default all types are considered.",
                ),
        )
        .arg(
            Arg::with_name("no_well_known")
                .long("no-well-known")
                .help("Do not look for debug symbols in well known locations."),
        )
        .arg(
            Arg::with_name("no_cwd")
                .long("no-cwd")
                .help("Do not look for debug symbols in the current working directory."),
        )
        .arg(
            Arg::with_name("paths")
                .long("path")
                .short("p")
                .multiple(true)
                .number_of_values(1)
                .help("Add a path to search recursively for debug info files."),
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .help("Format outputs as JSON."),
        )
        .arg(
            Arg::with_name("ids")
                .index(1)
                .value_name("ID")
                .help("The debug identifiers of the files to search for.")
                .validator(validate_id)
                .multiple(true)
                .number_of_values(1),
        )
}

fn id_hint(id: &DebugId) -> &'static str {
    if id.appendix() > 0 {
        return "likely PDB";
    }

    match id.uuid().get_version() {
        Some(UuidVersion::Sha1) => "likely Proguard",
        Some(UuidVersion::Md5) => "likely dSYM",
        None => "likely ELF Debug",
        _ => "unknown",
    }
}

fn find_ids(
    paths: HashSet<PathBuf>,
    types: HashSet<DifType>,
    ids: HashSet<DebugId>,
    as_json: bool,
) -> Result<bool, Error> {
    let mut remaining = ids.clone();
    let mut proguard_uuids: HashSet<_> = ids.iter()
        .map(|x| x.uuid())
        .filter(|&x| x.get_version() == Some(UuidVersion::Sha1))
        .collect();

    let iter = paths
        .iter()
        .flat_map(|p| WalkDir::new(p))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    let mut found_files = vec![];
    let pb = ProgressBar::new_spinner();
    pb.set_draw_target(ProgressDrawTarget::stdout());
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template(
                "{spinner} Looking for debug info files... {msg:.dim}\
                 \n  debug info files found: {prefix:.yellow}",
            ),
    );

    for dirent in iter {
        if remaining.is_empty() {
            break;
        }

        if let Some(p) = dirent.file_name().to_str() {
            pb.set_message(&p);
        }
        pb.tick();
        pb.set_prefix(&format!("{}", found_files.len()));

        let mut found = vec![];

        // specifically look for proguard files.  We only look for UUID5s
        // and only if the file is a text file.
        if_chain! {
            if !proguard_uuids.is_empty();
            if types.contains(&DifType::Proguard);
            if dirent.path().extension() == Some(OsStr::new("txt"));
            if let Ok(md) = dirent.metadata();
            if md.len() < MAX_MAPPING_FILE;
            if let Ok(byteview) = ByteView::from_path(dirent.path());
            if let Ok(mapping) = ProguardMappingView::parse(byteview);
            if proguard_uuids.contains(&mapping.uuid());
            then {
                found.push((mapping.uuid().into(), DifType::Proguard));
            }
        }

        // look for dsyms
        if_chain! {
            if types.contains(&DifType::Dsym);
            // we regularly match on .class files but the will never be
            // dsyms, so we can quickly skip them here
            if dirent.path().extension() != Some(OsStr::new("class"));
            if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Dsym));
            then {
                for id in dif.ids() {
                    if remaining.contains(&id) {
                        found.push((id, DifType::Dsym));
                    }
                }
            }
        }

        // look for breakpad files
        if_chain! {
            if types.contains(&DifType::Breakpad);
            if dirent.path().extension() == Some(OsStr::new("sym"));
            if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Breakpad));
            then {
                for id in dif.ids() {
                    if remaining.contains(&id) {
                        found.push((id, DifType::Breakpad));
                    }
                }
            }
        }

        for (id, ty) in found {
            found_files.push(DifMatch {
                ty: ty,
                id: id,
                path: dirent.path().to_path_buf(),
            });
            remaining.remove(&id);
            proguard_uuids.remove(&id.uuid());
        }
    }

    pb.finish_and_clear();

    if as_json {
        serde_json::to_writer_pretty(&mut io::stdout(), &found_files)?;
        println!("");
    } else {
        for m in found_files {
            println!(
                "{} {} [{}]",
                style(m.id).dim(),
                m.path.display(),
                style(m.ty).yellow()
            );
        }
        if !remaining.is_empty() {
            println_stderr!("");
            println_stderr!("missing debug information files:");
            for id in &remaining {
                println_stderr!("  {} ({})", id, id_hint(&id),);
            }
        }
    }

    Ok(remaining.is_empty())
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let mut paths = HashSet::new();
    let mut types = HashSet::new();
    let mut ids = HashSet::new();

    // which types should we consider?
    if let Some(t) = matches.values_of("types") {
        for ty in t {
            types.insert(ty.parse().unwrap());
        }
    } else {
        types.insert(DifType::Dsym);
        types.insert(DifType::Breakpad);
        types.insert(DifType::Proguard);
    }

    let with_well_known = !matches.is_present("no_well_known");
    let with_cwd = !matches.is_present("no_cwd");

    // start adding well known locations
    if_chain! {
        if with_well_known;
        if types.contains(&DifType::Dsym);
        if let Some(path) = env::home_dir().map(|x| x.join("Library/Developer/Xcode/DerivedData"));
        if path.is_dir();
        then {
            paths.insert(path);
        }
    }

    // current folder if wanted
    if_chain! {
        if with_cwd;
        if let Ok(path) = env::current_dir();
        then {
            paths.insert(path);
        }
    }

    // extra paths
    if let Some(p) = matches.values_of("paths") {
        for path in p {
            paths.insert(PathBuf::from(path));
        }
    }

    // which ids are we looking for?
    if let Some(i) = matches.values_of("ids") {
        for id in i {
            ids.insert(id.parse().unwrap());
        }
    } else {
        return Ok(());
    }

    if !find_ids(paths, types, ids, matches.is_present("json"))? {
        return Err(QuietExit(1).into());
    }

    Ok(())
}
