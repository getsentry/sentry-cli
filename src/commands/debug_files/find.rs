use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use clap::{builder::PossibleValuesParser, Arg, ArgAction, ArgMatches, Command};
use console::style;
use if_chain::if_chain;
use proguard::ProguardMapping;
use serde::Serialize;
use symbolic::common::{ByteView, DebugId};
use uuid::{Uuid, Version as UuidVersion};
use walkdir::{DirEntry, WalkDir};

use crate::utils::dif::{DifFile, DifType};
use crate::utils::progress::{ProgressBar, ProgressStyle};
use crate::utils::system::QuietExit;

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

pub fn make_command(command: Command) -> Command {
    command
        .about("Locate debug information files for given debug identifiers.")
        .arg(
            Arg::new("ids")
                .value_name("ID")
                .help("The debug identifiers of the files to search for.")
                .value_parser(DebugId::from_str)
                .multiple_values(true)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("types")
                .long("type")
                .short('t')
                .value_name("TYPE")
                .action(ArgAction::Append)
                .value_parser(PossibleValuesParser::new(DifType::all_names()))
                .help(
                    "Only consider debug information files of the given \
                     type.  By default all types are considered.",
                ),
        )
        .arg(
            Arg::new("no_well_known")
                .long("no-well-known")
                .help("Do not look for debug symbols in well known locations."),
        )
        .arg(
            Arg::new("no_cwd")
                .long("no-cwd")
                .help("Do not look for debug symbols in the current working directory."),
        )
        .arg(
            Arg::new("paths")
                .long("path")
                .short('p')
                .value_name("PATH")
                .action(ArgAction::Append)
                .help("Add a path to search recursively for debug info files."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Format outputs as JSON."),
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
    paths: &HashSet<PathBuf>,
    types: &HashSet<DifType>,
    ids: &HashSet<DebugId>,
    as_json: bool,
) -> Result<bool> {
    let mut remaining = ids.clone();
    let mut breakpad_found = HashSet::new();
    let mut proguard_uuids: HashSet<_> = ids
        .iter()
        .map(DebugId::uuid)
        .filter(|&x| x.get_version() == Some(UuidVersion::Sha1))
        .collect();

    let iter = paths
        .iter()
        .flat_map(WalkDir::new)
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file());

    let mut found_files = vec![];
    let pb = ProgressBar::new_spinner();
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
            pb.set_message(p);
        }
        pb.tick();
        pb.set_prefix(&format!("{}", found_files.len()));

        let found: Vec<_> = types
            .iter()
            .filter_map(|t| match t {
                DifType::Dsym => find_ids_for_dsym(&dirent, &remaining),
                DifType::Elf => find_ids_for_elf(&dirent, &remaining),
                DifType::Pe => find_ids_for_pe(&dirent, &remaining),
                DifType::Pdb => find_ids_for_pdb(&dirent, &remaining),
                DifType::PortablePdb => find_ids_for_portablepdb(&dirent, &remaining),
                DifType::SourceBundle => find_ids_for_sourcebundle(&dirent, &remaining),
                DifType::Breakpad => find_ids_for_breakpad(&dirent, &remaining),
                DifType::Proguard => find_ids_for_proguard(&dirent, &proguard_uuids),
                DifType::Wasm => None,
            })
            .flatten()
            .collect();

        for (id, ty) in found {
            let path = dirent.path().to_path_buf();
            found_files.push(DifMatch { ty, id, path });
            if ty == DifType::Breakpad {
                breakpad_found.insert(id);
            } else {
                remaining.remove(&id);
            }
            proguard_uuids.remove(&id.uuid());
        }
    }

    pb.finish_and_clear();

    if as_json {
        serde_json::to_writer_pretty(&mut io::stdout(), &found_files)?;
        println!();
    } else {
        for m in found_files {
            println!(
                "{} {} [{}]",
                style(m.id).dim(),
                m.path.display(),
                style(m.ty).yellow()
            );
        }
        remaining.extend(breakpad_found);
        if !remaining.is_empty() {
            eprintln!();
            eprintln!("missing debug information files:");
            for id in &remaining {
                eprintln!("  {} ({})", id, id_hint(id),);
            }
        }
    }

    Ok(remaining.is_empty())
}

fn find_ids_for_proguard(
    dirent: &DirEntry,
    proguard_uuids: &HashSet<Uuid>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if !proguard_uuids.is_empty();
        if dirent.path().extension() == Some(OsStr::new("txt"));
        if let Ok(md) = dirent.metadata();
        if md.len() < MAX_MAPPING_FILE;
        if let Ok(byteview) = ByteView::open(dirent.path());
        let mapping = ProguardMapping::new(&byteview);
        if mapping.is_valid();
        if proguard_uuids.contains(&mapping.uuid());
        then {
            return Some(vec![(mapping.uuid().into(), DifType::Proguard)]);
        }
    }
    None
}

fn find_ids_for_dsym(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        // we regularly match on .class files but the will never be
        // dsyms, so we can quickly skip them here
        if dirent.path().extension() != Some(OsStr::new("class"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Dsym));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::Dsym))
        }
    }
    None
}

fn find_ids_for_elf(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Elf));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::Elf))
        }
    }
    None
}

fn find_ids_for_pe(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if dirent.path().extension() == Some(OsStr::new("exe")) ||
        dirent.path().extension() == Some(OsStr::new("dll"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Pe));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::Pe))
        }
    }
    None
}

fn find_ids_for_pdb(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if dirent.path().extension() == Some(OsStr::new("pdb"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Pdb));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::Pdb))
        }
    }
    None
}

fn find_ids_for_portablepdb(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if dirent.path().extension() == Some(OsStr::new("pdb"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::PortablePdb));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::PortablePdb))
        }
    }
    None
}

fn find_ids_for_sourcebundle(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if dirent.path().extension() == Some(OsStr::new("zip"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::SourceBundle));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::SourceBundle))
        }
    }
    None
}

fn find_ids_for_breakpad(
    dirent: &DirEntry,
    remaining: &HashSet<DebugId>,
) -> Option<Vec<(DebugId, DifType)>> {
    if_chain! {
        if dirent.path().extension() == Some(OsStr::new("sym"));
        if let Ok(dif) = DifFile::open_path(dirent.path(), Some(DifType::Breakpad));
        then {
            return Some(extract_remaining_ids(&dif.ids(), remaining, DifType::Breakpad))
        }
    }
    None
}

fn extract_remaining_ids(
    ids: &[DebugId],
    remaining: &HashSet<DebugId>,
    t: DifType,
) -> Vec<(DebugId, DifType)> {
    ids.iter()
        .filter_map(|id| {
            if remaining.contains(id) {
                return Some((id.to_owned(), t));
            }
            None
        })
        .collect()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let mut paths = HashSet::new();
    let mut types = HashSet::new();
    let mut ids = HashSet::new();

    // which types should we consider?
    if let Some(t) = matches.get_many::<String>("types") {
        for ty in t {
            types.insert(ty.parse().unwrap());
        }
    } else {
        types.extend(DifType::all());
    }

    let with_well_known = !matches.contains_id("no_well_known");
    let with_cwd = !matches.contains_id("no_cwd");

    // start adding well known locations
    if_chain! {
        if with_well_known;
        if types.contains(&DifType::Dsym);
        if let Some(path) = dirs::home_dir().map(|x| x.join("Library/Developer/Xcode/DerivedData"));
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
    if let Some(p) = matches.get_many::<String>("paths") {
        for path in p {
            paths.insert(PathBuf::from(path));
        }
    }

    // which ids are we looking for?
    if let Some(i) = matches.get_many::<DebugId>("ids") {
        for id in i {
            ids.insert(*id);
        }
    } else {
        return Ok(());
    }

    if !find_ids(&paths, &types, &ids, matches.contains_id("json"))? {
        return Err(QuietExit(1).into());
    }

    Ok(())
}
