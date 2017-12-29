use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::iter::Fuse;
use std::path::{Path, PathBuf};

use clap::{App, AppSettings, Arg, ArgMatches};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use symbolic_common::{ByteView, ObjectKind};
use symbolic_debuginfo::FatObject;
use uuid::Uuid;
use walkdir::{DirEntry, IntoIter as WalkDirIter, WalkDir};
use zip::ZipWriter;
use zip::write::FileOptions;

use api::{Api, DSymFile};
use config::Config;
use prelude::*;
use utils::{ArgExt, copy_with_progress, get_sha1_checksum, invert_result, make_byte_progress_bar,
            TempFile, validate_uuid};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload breakpad symbols to a project.")
        .setting(AppSettings::Hidden)
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
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Do not trigger reprocessing after uploading."))
}

struct ProjectContext<'a> {
    api: Api<'a>,
    org: String,
    project: String,
}

impl<'a> ProjectContext<'a> {
    pub fn from_cli(matches: &ArgMatches, config: &'a Config) -> Result<ProjectContext<'a>> {
        let (org, project) = config.get_org_and_project(matches)?;

        Ok(ProjectContext {
            api: Api::new(config),
            org: org,
            project: project,
        })
    }
}

#[derive(Debug)]
struct Sym {
    path: PathBuf,
    checksum: String,
    name: String,
    size: u64,
}

type Batch = Vec<Sym>;

struct BatchIter<'a> {
    path: PathBuf,
    max_size: u64,
    uuids: Option<&'a HashSet<Uuid>>,
    found: &'a mut HashSet<Uuid>,
    iter: Fuse<WalkDirIter>,
}

impl<'a> BatchIter<'a> {
    pub fn new(
        path: PathBuf,
        max_size: u64,
        uuids: Option<&'a HashSet<Uuid>>,
        found: &'a mut HashSet<Uuid>,
    ) -> BatchIter<'a> {
        let iter = WalkDir::new(&path).into_iter().fuse();

        BatchIter {
            path: path,
            max_size: max_size,
            uuids: uuids,
            found: found,
            iter: iter,
        }
    }

    fn found_all(&self) -> bool {
        if let Some(ref uuids) = self.uuids {
            self.found.is_superset(uuids)
        } else {
            false
        }
    }

    fn is_filled(&self, batch: &Batch) -> bool {
        batch.iter().map(|sym| sym.size).sum::<u64>() >= self.max_size
    }

    fn process_file(&mut self, entry: DirEntry, pb: &ProgressBar) -> Result<Option<Sym>> {
        // The WalkDir iterator will automatically recurse into directories
        let meta = entry.metadata()?;
        if !meta.is_file() {
            return Ok(None);
        }

        // Require the usual "sym" extension for Breakpad symbols
        let path = entry.path();
        if path.extension() != Some(OsStr::new("sym")) {
            return Ok(None);
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            pb.set_message(name);
        }

        // Make sure that this is a breakpad file
        let data = ByteView::from_path(path)?;
        match FatObject::peek(&data) {
            Ok(ObjectKind::Breakpad) => {}
            _ => return Ok(None),
        };

        // Parse the object and make sure it contains a valid UUID
        let fat = FatObject::parse(data)?;
        let object = fat.get_object(0)?.unwrap();
        let uuid = match object.uuid() {
            Some(uuid) => uuid,
            None => return Ok(None),
        };

        // See if the UUID matches the provided UUIDs
        if !self.uuids.map_or(true, |uuids| uuids.contains(&uuid)) {
            return Ok(None);
        }

        let file_name = Path::new("DebugSymbols")
            .join(path.strip_prefix(&self.path).unwrap())
            .to_string_lossy()
            .into_owned();

        Ok(Some(Sym {
            path: path.to_path_buf(),
            checksum: get_sha1_checksum(object.as_bytes())?,
            name: file_name,
            size: meta.len(),
        }))
    }

    fn next_sym(&mut self, pb: &ProgressBar) -> Result<Option<Sym>> {
        while let Some(entry) = self.iter.next() {
            if let Some(sym) = self.process_file(entry?, pb)? {
                return Ok(Some(sym));
            }
        }

        Ok(None)
    }

    fn next_batch(&mut self) -> Result<Option<Batch>> {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(100);
        pb.set_style(ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner} Looking for symbols... {msg:.dim}\
                       \n  symbol files found: {prefix:.yellow}"));

        let mut batch = vec![];
        while !self.is_filled(&batch) && !self.found_all() {
            match self.next_sym(&pb)? {
                Some(sym) => {
                    batch.push(sym);
                    pb.set_prefix(&format!("{}", batch.len()));
                },
                None => break,
            }
        }

        pb.finish_and_clear();
        Ok(if batch.len() == 0 {
            None
        } else {
            Some(batch)
        })
    }
}

impl<'a> Iterator for BatchIter<'a> {
    type Item = Result<Batch>;

    fn next(&mut self) -> Option<Self::Item> {
        invert_result(self.next_batch())
    }
}

fn filter_missing_syms(batch: Batch, context: &mut ProjectContext) -> Result<Batch> {
    info!("Checking for missing debug symbols: {:#?}", &batch);

    let missing_checksums = {
        let checksums = batch.iter().map(|ref s| s.checksum.as_str()).collect();
        context.api.find_missing_dsym_checksums(&context.org, &context.project, &checksums)?
    };

    let missing = batch.into_iter()
        .filter(|sym| missing_checksums.contains(&sym.checksum))
        .collect();

    info!("Missing debug symbols: {:#?}", &missing);
    Ok(missing)
}

fn compress_syms(batch: Batch) -> Result<TempFile> {
    let total_bytes = batch.iter().map(|sym| sym.size).sum();
    let pb = make_byte_progress_bar(total_bytes);
    let tf = TempFile::new()?;
    let mut zip = ZipWriter::new(tf.open());

    for ref sym in batch {
        zip.start_file(sym.name.clone(), FileOptions::default())?;
        copy_with_progress(&pb, &mut File::open(&sym.path)?, &mut zip)?;
    }

    pb.finish_and_clear();
    Ok(tf)
}

fn upload_syms(batch: Batch, context: &mut ProjectContext) -> Result<Vec<DSymFile>> {
    println!(
        "{} Compressing {} missing debug symbol files",
        style(">").dim(),
        style(batch.len()).yellow()
    );
    let archive = compress_syms(batch)?;

    println!("{} Uploading debug symbol files", style(">").dim());
    Ok(context.api.upload_dsyms(&context.org, &context.project, archive.path())?)
}

fn process_batch(batch: Batch, context: &mut ProjectContext) -> Result<usize> {
    println!(
        "{} Found {} breakpad symbol files.",
        style(">").dim(),
        style(batch.len()).yellow()
    );

    let missing = filter_missing_syms(batch, context)?;
    if missing.len() == 0 {
        println!("{} Nothing to compress, all symbols are on the server", style(">").dim());
        println!("{} Nothing to upload", style(">").dim());
        return Ok(0);
    }

    let uploaded = upload_syms(missing, context)?;
    if uploaded.len() > 0 {
        println!("Newly uploaded debug symbols:");
        for sym in &uploaded {
            println!("  {} ({}; {})", style(&sym.uuid).dim(), &sym.object_name, sym.cpu_name);
        }
    }

    Ok(uploaded.len())
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let paths = match matches.values_of("paths") {
        Some(paths) => paths.map(|path| PathBuf::from(path)).collect(),
        None => vec![],
    };

    if paths.len() == 0 {
        // We allow this because reprocessing will still be triggered
        println!("Warning: no paths were provided.");
    }

    let mut found = HashSet::new();
    let uuids = matches.values_of("uuids").map(|uuids| {
        uuids.map(|s| Uuid::parse_str(s).unwrap()).collect()
    });

    let mut context = ProjectContext::from_cli(matches, config)?;
    let max_size = config.get_max_dsym_upload_size()?;
    let mut total_uploaded = 0;

    // Search all paths and upload symbols in batches
    for path in paths.into_iter() {
        let iter = BatchIter::new(path, max_size, uuids.as_ref(), &mut found);
        for (i, batch) in iter.enumerate() {
            if i > 0 {
                println!("");
            }

            println!("{}", style(format!("Batch {}", i)).bold());
            total_uploaded += process_batch(batch?, &mut context)?;
        }
    }

    if total_uploaded > 0 {
        println!("Uploaded a total of {} breakpad symbols", style(total_uploaded).yellow());
    }

    // Trigger reprocessing only if requested by user
    if !matches.is_present("no_reprocessing") {
        if !context.api.trigger_reprocessing(&context.org, &context.project)? {
            println!("{} Server does not support reprocessing. Not triggering.", style(">").dim());
        }
    } else {
        println!("{} skipped reprocessing", style(">").dim());
    }

    // did we miss explicitly requested symbols?
    if matches.is_present("require_all") {
        if let Some(ref uuids) = uuids {
            let missing: HashSet<_> = uuids.difference(&found).collect();
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
    }

    Ok(())
}
