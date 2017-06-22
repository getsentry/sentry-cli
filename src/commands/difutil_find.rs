use std::io;
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::collections::HashSet;

use clap::{App, Arg, ArgMatches};
use uuid::{Uuid, UuidVersion};
use walkdir::WalkDir;
use proguard;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use console::style;
use serde_json;

use prelude::*;
use config::Config;
use utils::{validate_uuid, MachoInfo, dif};

// text files larger than 32 megabytes are not considered to be
// valid mapping files when scanning
const MAX_MAPPING_FILE: u64 = 32 * 1024 * 1024;

#[derive(Serialize, Debug)]
struct DifMatch {
    #[serde(rename="type")]
    pub ty: dif::DifType,
    pub uuid: Uuid,
    pub path: PathBuf,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app
        .about("given UUIDs this locates debug information files")
        .arg(Arg::with_name("types")
             .long("type")
             .short("t")
             .value_name("TYPE")
             .multiple(true)
             .number_of_values(1)
             .possible_values(&["dsym", "proguard"])
             .help("Only consider debug information files of the given \
                    type.  By default all types are considered."))
        .arg(Arg::with_name("no_well_known")
             .long("no-well-known")
             .help("Do not look for debug symbols in well known locations."))
        .arg(Arg::with_name("no_cwd")
             .long("no-cwd")
             .help("Do not look for debug symbols starting from the current \
                    working directory."))
        .arg(Arg::with_name("paths")
             .long("path")
             .short("p")
             .multiple(true)
             .number_of_values(1)
             .help("Adds a starting point for searching for debug info files."))
        .arg(Arg::with_name("json")
             .long("json")
             .help("Returns the results as JSON"))
        .arg(Arg::with_name("uuids")
             .index(1)
             .value_name("UUID")
             .help("Finds debug information files by UUID.")
             .validator(validate_uuid)
             .multiple(true)
             .number_of_values(1))
}

fn find_uuids(paths: HashSet<PathBuf>,
              types: HashSet<dif::DifType>,
              uuids: HashSet<Uuid>,
              as_json: bool) -> Result<bool>
{
    let mut remaining = uuids.clone();
    let mut proguard_uuids: HashSet<_> = uuids
        .iter()
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
    pb.set_style(ProgressStyle::default_spinner()
        .tick_chars("/|\\- ")
        .template("{spinner} Looking for debug info files... {msg:.dim}\
                   \n  debug info files found: {prefix:.yellow}"));

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
            if types.contains(&dif::DifType::Proguard);
            if dirent.path().extension() == Some(OsStr::new("txt"));
            if let Ok(mapping) = proguard::MappingView::from_path(dirent.path());
            if proguard_uuids.contains(&mapping.uuid());
            then {
                found.push((mapping.uuid(), dif::DifType::Proguard));
            }
        }

        // look for dsyms
        if_chain! {
            if types.contains(&dif::DifType::Dsym);
            // we regularly match on .class files but the will never be
            // dsyms, so we can quickly skip them here
            if dirent.path().extension() != Some(OsStr::new("class"));
            if let Ok(md) = dirent.metadata();
            if md.len() < MAX_MAPPING_FILE;
            if let Ok(info) = MachoInfo::open_path(dirent.path());
            if info.matches_any(&remaining);
            then {
                for uuid in info.get_uuids() {
                    if remaining.contains(&uuid) {
                        found.push((uuid, dif::DifType::Dsym));
                    }
                }
            }
        }

        for (uuid, ty) in found {
            found_files.push(DifMatch {
                ty: ty,
                uuid: uuid,
                path: dirent.path().to_path_buf(),
            });
            remaining.remove(&uuid);
            proguard_uuids.remove(&uuid);
        }
    }

    pb.finish_and_clear();

    if as_json {
        serde_json::to_writer_pretty(&mut io::stdout(), &found_files)?;
        println!("");
    } else {
        for m in found_files {
            println!("{} {} [{}]", style(m.uuid).dim(), m.path.display(), style(m.ty).yellow());
        }
        if !remaining.is_empty() {
            println_stderr!("");
            println_stderr!("missing debug information files:");
            for uuid in &remaining {
                println_stderr!("  {} ({})", uuid, match uuid.get_version() {
                    Some(UuidVersion::Sha1) => "likely proguard",
                    Some(UuidVersion::Md5) => "likely dsym",
                    _ => "unknown",
                });
            }
        }
    }

    Ok(remaining.is_empty())
}

pub fn execute<'a>(matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    let mut paths = HashSet::new();
    let mut types = HashSet::new();
    let mut uuids = HashSet::new();

    // which types should we consider?
    if let Some(t) = matches.values_of("types") {
        for ty in t {
            types.insert(ty.parse().unwrap());
        }
    } else {
        types.insert(dif::DifType::Dsym);
        types.insert(dif::DifType::Proguard);
    }

    let with_well_known = !matches.is_present("no_well_known");
    let with_cwd = !matches.is_present("no_cwd");

    // start adding well known locations
    if_chain! {
        if with_well_known;
        if types.contains(&dif::DifType::Dsym);
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

    // which uuids are we looking for?
    if let Some(u) = matches.values_of("uuids") {
        for uuid in u {
            uuids.insert(uuid.parse().unwrap());
        }
    } else {
        return Ok(());
    }

    if !find_uuids(paths, types, uuids, matches.is_present("json"))? {
        return Err(ErrorKind::QuietExit(1).into());
    }

    Ok(())
}
