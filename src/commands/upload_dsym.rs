//! Implements a command for uploading dsym files.
use std::io;
use std::fs;
use std::env;
use std::str;
use std::fmt;
use std::process;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Seek, Write};
use std::ffi::OsStr;
use std::cell::RefCell;
use std::iter::Fuse;
use std::rc::Rc;
use std::collections::HashSet;

use clap::{App, Arg, ArgMatches};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::Digest;
use symbolic_common::{ByteView, DebugKind, ObjectClass, ObjectKind};
use symbolic_debuginfo::FatObject;
use uuid::Uuid;
use walkdir::{IntoIter as WalkDirIter, WalkDir};
use which;
use zip;

use api::{Api, DSymFile};
use config::Config;
use utils::dif::has_hidden_symbols;
use prelude::*;
use utils::{copy_with_progress, is_zip_file, make_byte_progress_bar, validate_uuid, xcode, ArgExt,
            TempDir, TempFile, get_sha1_checksum};

#[derive(Debug)]
enum DSymVar {
    FsFile(PathBuf),
    TempFile(TempFile),
    ZipFile(Rc<RefCell<Option<zip::ZipArchive<fs::File>>>>, usize),
}

struct DSymRef {
    var: DSymVar,
    arc_name: String,
    checksum: Digest,
    size: u64,
    uuids: Vec<Uuid>,
    has_hidden_symbols: bool,
}

impl fmt::Debug for DSymRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DSymRef")
            .field("arc_name", &self.arc_name)
            .field("checksum", &self.checksum)
            .field("size", &self.size)
            .field("uuids", &self.uuids)
            .field("has_hidden_symbols", &self.has_hidden_symbols)
            .finish()
    }
}

impl DSymRef {
    pub fn add_to_archive<W: Write + Seek>(&self, mut zip: &mut zip::ZipWriter<W>,
                                           pb: &ProgressBar) -> Result<()> {
        zip.start_file(self.arc_name.clone(), zip::write::FileOptions::default())?;
        match self.var {
            DSymVar::FsFile(ref p) => {
                copy_with_progress(pb, &mut File::open(&p)?, &mut zip)?;
            }
            DSymVar::TempFile(ref p) => {
                copy_with_progress(pb, &mut p.open(), &mut zip)?;
            }
            DSymVar::ZipFile(ref rc, idx) => {
                let rc = rc.clone();
                let mut opt_archive = rc.borrow_mut();
                if let Some(ref mut archive) = *opt_archive {
                    let mut af = archive.by_index(idx)?;
                    copy_with_progress(pb, &mut af, &mut zip)?;
                } else {
                    panic!("zip file went away");
                }
            }
        }
        Ok(())
    }

    pub fn dsym_name(&self) -> &str {
        if_chain! {
            if let Some(filename_os) = Path::new(&self.arc_name).file_name();
            if let Some(filename) = filename_os.to_str();
            then {
                filename
            } else {
                "Generic"
            }
        }
    }

    pub fn is_swift_support(&self) -> bool {
        self.dsym_name().starts_with("libswift")
    }

    pub fn resolve_bcsymbolmaps(&mut self, symbol_map_path: &Path) -> Result<()> {
        let td = TempDir::new()?;
        fs::create_dir_all(td.path().join("DWARF"))?;
        let mut df = fs::File::create(td.path().join("DWARF").join(self.dsym_name()))?;
        let mut zipname = None;

        // copy the dsym contents over
        match self.var {
            DSymVar::FsFile(ref p) => {
                io::copy(&mut File::open(&p)?, &mut df)?;
            }
            DSymVar::TempFile(..) => {
                fail!("Cannot resolve BCSymbolMaps in temporary files.");
            }
            DSymVar::ZipFile(ref rc, idx) => {
                let rc = rc.clone();
                let mut opt_archive = rc.borrow_mut();
                if let Some(ref mut archive) = *opt_archive {
                    let mut af = archive.by_index(idx)?;
                    zipname = Some(PathBuf::from(af.name()));
                    io::copy(&mut af, &mut df)?;
                } else {
                    panic!("zip file went away");
                }
            }
        }

        // place the debug references
        for uuid in &self.uuids {
            let plist_ref = format!("{}.plist", uuid.to_string().to_uppercase());
            match self.var {
                DSymVar::FsFile(ref p) => {
                    if_chain! {
                        if let Some(base) = p.parent().and_then(|x| x.parent());
                        if let Ok(mut f) = fs::File::open(base.join(&plist_ref));
                        then {
                            io::copy(&mut f, &mut fs::File::create(td.path().join(&plist_ref))?)?;
                        }
                    }
                }
                DSymVar::TempFile(..) => {
                    fail!("Cannot resolve BCSymbolMaps in temporary files.");
                }
                DSymVar::ZipFile(ref rc, ..) => {
                    let rc = rc.clone();
                    let mut opt_archive = rc.borrow_mut();
                    if let Some(ref mut archive) = *opt_archive {
                        let mut af = archive.by_name(
                            zipname.as_ref().unwrap().join(&plist_ref).to_str().unwrap())?;
                        let mut df = fs::File::open(td.path().join(&plist_ref))?;
                        io::copy(&mut af, &mut df)?;
                    } else {
                        panic!("zip file went away");
                    }
                }
            }
        }

        // invoke dsymutil
        let p = process::Command::new("dsymutil")
            .arg("-symbol-map")
            .arg(symbol_map_path)
            .arg(&td.path().join("DWARF").join(self.dsym_name()))
            .output()?;
        if !p.status.success() {
            if let Ok(msg) = str::from_utf8(&p.stderr) {
                fail!("Could not resolve BCSymbolMaps: {}", msg);
            } else {
                fail!("Could not resolve BCSymbolMaps due to an unknown error");
            }
        }

        // replace us with the new tempfile
        let tf = TempFile::new()?;
        io::copy(&mut fs::File::open(&td.path().join("DWARF").join(self.dsym_name()))?,
                 &mut tf.open())?;
        self.has_hidden_symbols = false;
        self.checksum = get_sha1_checksum(&mut tf.open())?;
        self.size = tf.size()?;
        self.var = DSymVar::TempFile(tf);

        Ok(())
    }
}

struct BatchIter<'a> {
    path: PathBuf,
    max_size: u64,
    wd_iter: Fuse<WalkDirIter>,
    open_zip: Rc<RefCell<Option<zip::ZipArchive<fs::File>>>>,
    open_zip_index: usize,
    uuids: Option<&'a HashSet<Uuid>>,
    allow_zips: bool,
    found_uuids: RefCell<&'a mut HashSet<Uuid>>,
}

impl<'a> BatchIter<'a> {
    pub fn new<P: AsRef<Path>>(path: P, max_size: u64, uuids: Option<&'a HashSet<Uuid>>,
                               allow_zips: bool, found_uuids: &'a mut HashSet<Uuid>)
        -> BatchIter<'a>
    {
        BatchIter {
            path: path.as_ref().to_path_buf(),
            max_size: max_size,
            wd_iter: WalkDir::new(&path).into_iter().fuse(),
            open_zip: Rc::new(RefCell::new(None)),
            open_zip_index: !0,
            uuids: uuids,
            allow_zips: allow_zips,
            found_uuids: RefCell::new(found_uuids),
        }
    }

    fn found_all(&self) -> bool {
        if let Some(ref uuids) = self.uuids {
            self.found_uuids.borrow().is_superset(uuids)
        } else {
            false
        }
    }

    fn push_ref(&self, batch: &mut Vec<DSymRef>, dsym_ref: DSymRef) -> bool {
        let mut found_uuids = self.found_uuids.borrow_mut();
        let mut should_push = false;
        for uuid in &dsym_ref.uuids {
            if found_uuids.contains(uuid) {
                continue;
            }
            should_push = true;
            found_uuids.insert(*uuid);
        }
        if should_push {
            batch.push(dsym_ref);
        }
        batch.iter().map(|x| x.size).sum::<u64>() >= self.max_size
    }

    fn matches_uuids(&self, fat_object: &FatObject) -> Result<bool> {
        let uuids = match self.uuids {
            Some(uuids) => uuids,
            None => return Ok(true),
        };

        for object_result in fat_object.objects() {
            let object = object_result?;
            let is_dsym = object.kind() == ObjectKind::MachO
                && object.class() == ObjectClass::Debug
                && object.debug_kind() == Some(DebugKind::Dwarf);

            if !is_dsym {
                continue;
            }

            if let Some(uuid) = object.uuid() {
                if uuids.contains(&uuid) {
                    return Ok(true);
                }
            }
        }

        return Ok(false);
    }

    fn parse_matching_object<'data>(
        &self,
        data: ByteView<'data>,
    ) -> Result<Option<FatObject<'data>>> {
        match FatObject::peek(&data) {
            Ok(ObjectKind::MachO) => {}
            _ => return Ok(None),
        };

        let fat = FatObject::parse(data)?;
        if self.matches_uuids(&fat)? {
            Ok(Some(fat))
        } else {
            Ok(None)
        }
    }

    fn next_batch(&mut self) -> Result<Option<Vec<DSymRef>>> {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(100);
        pb.set_style(ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner} Looking for symbols... {msg:.dim}\
                       \n  symbol files found: {prefix:.yellow}"));

        let mut batch = vec![];

        while !self.found_all() {
            if self.open_zip_index == !0 {
                *self.open_zip.borrow_mut() = None;
            }

            if self.open_zip_index != !0 {
                let mut archive_ptr = self.open_zip.borrow_mut();
                let archive = archive_ptr.as_mut().unwrap();
                if self.open_zip_index >= archive.len() {
                    self.open_zip_index = !0;
                    if batch.len() != 0 {
                        break;
                    }
                } else {
                    let zip_file = archive.by_index(self.open_zip_index)?;
                    let zip_name = Path::new("DebugSymbols").join(zip_file.name());
                    let data = ByteView::from_reader(zip_file)?;
                    if let Some(object) = self.parse_matching_object(data)? {
                        let is_full = self.push_ref(&mut batch, DSymRef {
                            var: DSymVar::ZipFile(self.open_zip.clone(), self.open_zip_index),
                            arc_name: zip_name.to_string_lossy().into_owned(),
                            checksum: get_sha1_checksum(object.as_bytes())?,
                            size: object.as_bytes().len() as u64,
                            uuids: collect_uuids(&object)?,
                            has_hidden_symbols: has_hidden_symbols(&object)?,
                        });

                        if is_full {
                            break;
                        }
                    }

                    self.open_zip_index += 1;
                }
            } else if let Some(entry_result) = self.wd_iter.next() {
                let entry = entry_result?;
                let meta = entry.metadata()?;
                if meta.is_file() {
                    if let Some(fname) = entry.path().file_name().and_then(|x| x.to_str()) {
                        pb.set_message(fname);
                    }

                    pb.set_prefix(&format!("{}", batch.len()));
                    if self.allow_zips && is_zip_file(fs::File::open(&entry.path())?) {
                        let f = fs::File::open(entry.path())?;
                        if let Ok(archive) = zip::ZipArchive::new(f) {
                            *self.open_zip.borrow_mut() = Some(archive);
                            self.open_zip_index = 0;
                            // whenever we switch the zip we need to yield because we
                            // might have references to an earlier zip
                            if batch.len() > 0 {
                                break;
                            }
                        }
                    } else {
                        let data = ByteView::from_path(entry.path())?;
                        if let Some(object) = self.parse_matching_object(data)? {
                            let name = Path::new("DebugSymbols")
                                .join(entry.path().strip_prefix(&self.path).unwrap());

                            let is_full = self.push_ref(&mut batch, DSymRef {
                                var: DSymVar::FsFile(entry.path().to_path_buf()),
                                arc_name: name.to_string_lossy().into_owned(),
                                checksum: get_sha1_checksum(object.as_bytes())?,
                                size: meta.len(),
                                uuids: collect_uuids(&object)?,
                                has_hidden_symbols: has_hidden_symbols(&object)?,
                            });

                            if is_full {
                                break;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        pb.finish_and_clear();
        if batch.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(batch))
        }
    }
}

impl<'a> Iterator for BatchIter<'a> {
    type Item = Result<Vec<DSymRef>>;

    fn next(&mut self) -> Option<Result<Vec<DSymRef>>> {
        match self.next_batch() {
            Ok(Some(batch)) => Some(Ok(batch)),
            Err(err) => Some(Err(err)),
            Ok(None) => None,
        }
    }
}

fn collect_uuids(fat: &FatObject) -> Result<Vec<Uuid>> {
    fat.objects().map(|o| Ok(o?.uuid().unwrap())).collect()
}

fn find_missing_files(api: &mut Api,
                      refs: Vec<DSymRef>,
                      org: &str,
                      project: &str)
                      -> Result<Vec<DSymRef>> {
    info!("Checking for missing debug symbols: {:#?}", &refs);
    let missing = {
        let checksums = refs.iter().map(|ref x| x.checksum);
        api.find_missing_dsym_checksums(org, project, checksums)?
    };
    let mut rv = vec![];
    for r in refs.into_iter() {
        if missing.contains(&r.checksum) {
            rv.push(r)
        }
    }
    info!("Missing debug symbols: {:#?}", &rv);
    Ok(rv)
}

fn zip_up_missing(refs: &[DSymRef]) -> Result<TempFile> {
    println!("{} Compressing {} missing debug symbol files", style(">").dim(),
             style(refs.len()).yellow());
    let total_bytes = refs.iter().map(|x| x.size).sum();
    let pb = make_byte_progress_bar(total_bytes);
    let tf = TempFile::new()?;
    let mut zip = zip::ZipWriter::new(tf.open());
    for ref r in refs {
        r.add_to_archive(&mut zip, &pb)?;
    }
    pb.finish_and_clear();
    Ok(tf)
}

fn upload_dsyms(api: &mut Api,
                refs: &[DSymRef],
                org: &str,
                project: &str)
                -> Result<Vec<DSymFile>> {
    let tf = zip_up_missing(refs)?;
    println!("{} Uploading debug symbol files", style(">").dim());
    Ok(api.upload_dsyms(org, project, tf.path())?)
}

fn resolve_bcsymbolmaps(refs: &mut [DSymRef],
                        symbol_map_path: Option<&Path>) -> Result<()> {
    let mut hidden_symbols = vec![];
    for (idx, r) in refs.iter().enumerate() {
        // XXX: for now we just ignroe libswift because there are various
        // issues with that.  In particular there are never any BCSymbolMaps
        // generated for it and the DBGOriginalUUID in the plist is the UUID
        // of the original dsym file.
        //
        // I *think* what we would have to do here is to locate the original
        // library in the xcode distribution, then build a new non-fat dSYM
        // file from it and patch the the UUID.
        if r.has_hidden_symbols && !r.is_swift_support() {
            hidden_symbols.push(idx);
        }
    }
    if hidden_symbols.is_empty() {
        return Ok(());
    }

    if let Some(symbol_map_path) = symbol_map_path {
        println!("{} Resolving {} BCSourceMaps",
                 style(">").dim(), style(hidden_symbols.len()).yellow());

        let pb = ProgressBar::new(hidden_symbols.len() as u64);
        for idx in hidden_symbols.into_iter() {
            let r = &mut refs[idx];
            pb.inc(1);
            r.resolve_bcsymbolmaps(symbol_map_path)?;
        }
        pb.finish_and_clear();
    } else {
        println!("{} {}: found {} symbol files with hidden symbols (need BCSymbolMaps)",
                 style(">").dim(), style("warning").red(),
                 style(hidden_symbols.len()).yellow());
    }

    Ok(())
}

fn get_paths_from_env() -> Result<Vec<PathBuf>> {
    let mut rv = vec![];
    if let Some(base_path) = env::var_os("DWARF_DSYM_FOLDER_PATH") {
        info!("Getting path from DWARF_DSYM_FOLDER_PATH: {}",
              Path::new(&base_path).display());
        for entry in WalkDir::new(base_path) {
            let entry = entry?;
            if entry.path().extension() == Some(OsStr::new("dSYM")) &&
               fs::metadata(entry.path())?.is_dir() {
                rv.push(entry.path().to_path_buf());
            }
        }
    }
    Ok(rv)
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload Mac debug symbols to a project.")
        .org_project_args()
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("A path to search recursively for symbol files.")
            .multiple(true)
            .number_of_values(1)
            .index(1))
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
        .arg(Arg::with_name("symbol_maps")
             .long("symbol-maps")
             .value_name("PATH")
             .help("Optional path to bcsymbolmap files which are used to \
                    resolve hidden symbols in the actual dsym files.  This \
                    requires the dsymutil tool to be available."))
        .arg(Arg::with_name("derived_data")
             .long("derived-data")
             .help("Search for debug symbols in derived data."))
        .arg(Arg::with_name("no_zips")
             .long("no-zips")
             .help("Do not recurse into ZIP files."))
        .arg(Arg::with_name("info_plist")
             .long("info-plist")
             .value_name("PATH")
             .help("Optional path to the Info.plist.{n}We will try to find this \
                    automatically if run from xcode.  Providing this information \
                    will associate the debug symbols with a specific ITC application \
                    and build in Sentry.  Note that if you provide the plist \
                    explicitly it must already be processed."))
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Do not trigger reprocessing after uploading."))
        .arg(Arg::with_name("force_foreground")
             .long("force-foreground")
             .help("Wait for the process to finish.{n}\
                    By default the upload process will when triggered from Xcode \
                    detach and continue in the background.  When an error happens \
                    a dialog is shown.  If this parameter is passed Xcode will wait \
                    for the process to finish before the build finishes and output \
                    will be shown in the Xcode build output."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let zips = !matches.is_present("no_zips");
    let mut paths = match matches.values_of("paths") {
        Some(paths) => paths.map(|x| PathBuf::from(x)).collect(),
        None => get_paths_from_env()?,
    };
    let symbol_maps_path = match matches.value_of("symbol_maps") {
        Some(path) => {
            if which::which("dsymutil").is_err() {
                fail!("--symbol-maps requires the apple dsymutil to be available.");
            }
            Some(Path::new(path))
        }
        None => None,
    };
    if_chain! {
        if matches.is_present("derived_data");
        if let Some(path) = env::home_dir().map(|x| x.join("Library/Developer/Xcode/DerivedData"));
        if path.is_dir();
        then {
            paths = vec![path];
        }
    }
    let find_uuids = matches.values_of("uuids").map(|uuids| {
        uuids.map(|s| Uuid::parse_str(s).unwrap()).collect::<HashSet<_>>()
    });
    let mut found_uuids: HashSet<Uuid> = HashSet::new();
    let info_plist = match matches.value_of("info_plist") {
        Some(path) => Some(xcode::InfoPlist::from_path(path)?),
        None => xcode::InfoPlist::discover_from_env()?,
    };

    if paths.len() == 0 {
        println!("Warning: no paths were provided.");
    }

    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;
    let max_size = config.get_max_dsym_upload_size()?;
    let mut api = Api::new();
    let mut total_uploaded = 0;

    xcode::MayDetach::wrap("Debug symbol upload", |md| {
        // Optionally detach if run from xcode
        if !matches.is_present("force_foreground") {
            md.may_detach()?;
        }

        let mut batch_num = 0;
        let mut all_dsym_checksums = vec![];
        for path in paths.into_iter() {
            info!("Scanning {}", path.display());
            for batch_res in BatchIter::new(path, max_size, find_uuids.as_ref(),
                                            zips, &mut found_uuids) {
                if batch_num > 0 {
                    println!("");
                }
                batch_num += 1;
                let mut batch = batch_res?;
                println!("{}", style(format!("Batch {}", batch_num)).bold());
                println!("{} Found {} debug symbol files.",
                         style(">").dim(), style(batch.len()).yellow());
                resolve_bcsymbolmaps(&mut batch, symbol_maps_path)?;
                for dsym_ref in batch.iter() {
                    all_dsym_checksums.push(dsym_ref.checksum.to_string());
                }
                println!("{} Checking for missing debug symbol files on server",
                         style(">").dim());
                let missing = find_missing_files(&mut api, batch, &org, &project)?;
                if missing.len() == 0 {
                    println!("{} Nothing to compress, all symbols are on the server",
                             style(">").dim());
                    println!("{} Nothing to upload", style(">").dim());
                    continue;
                }
                let rv = upload_dsyms(&mut api, &missing, &org, &project)?;
                if rv.len() > 0 {
                    total_uploaded += rv.len();
                    println!("Newly uploaded debug symbols:");
                    for df in rv {
                        println!("  {} ({}; {})",
                                 style(&df.uuid).dim(),
                                 &df.object_name,
                                 df.cpu_name);
                    }
                }
            }
        }

        // associate the dsyms with the info plist data if available
        if let Some(ref info_plist) = info_plist {
            println!("Associating dsyms with {}", info_plist);
            match api.associate_apple_dsyms(&org, &project, info_plist, all_dsym_checksums)? {
                None => {
                    println!("Server does not support dsym associations. Ignoring.");
                }
                Some(resp) => {
                    if resp.associated_dsyms.len() == 0 {
                        println!("No new debug symbols to associate.");
                    } else {
                        println!("Associated {} debug symbols with the build.",
                                 style(resp.associated_dsyms.len()).yellow());
                    }
                }
            }
        }

        if total_uploaded > 0 {
            println!("Uploaded a total of {} debug symbols",
                     style(total_uploaded).yellow());
        }

        // If wanted trigger reprocessing
        if !matches.is_present("no_reprocessing") {
            if !api.trigger_reprocessing(&org, &project)? {
                println!("{} Server does not support reprocessing. Not triggering.",
                         style(">").dim());
            }
        } else {
            println!("{} skipped reprocessing", style(">").dim());
        }

        // did we miss anything?
        if let Some(ref find_uuids) = find_uuids {
            let missing: HashSet<_> = find_uuids.difference(&found_uuids).collect();
            if matches.is_present("require_all") && !missing.is_empty() {
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
    })
}
