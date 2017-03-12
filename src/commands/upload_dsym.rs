//! Implements a command for uploading dsym files.

use std::io;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::mem;
use std::io::{Write, Seek};
use std::ffi::OsStr;
use std::cell::RefCell;
use std::rc::Rc;

use clap::{App, Arg, ArgMatches};
use walkdir::{WalkDir, Iter as WalkDirIter};
use zip;

use prelude::*;
use api::{Api, DSymFile};
use utils::{TempFile, get_sha1_checksum, is_zip_file};
use macho::is_macho_file;
use config::Config;

const BATCH_SIZE: usize = 15;

enum DSymVar {
    FsFile(PathBuf),
    ZipFile(Rc<RefCell<Option<zip::ZipArchive<fs::File>>>>, usize),
}

struct DSymRef {
    var: DSymVar,
    arc_name: String,
    checksum: String,
}

impl DSymRef {
    pub fn add_to_archive<W: Write + Seek>(&self, mut zip: &mut zip::ZipWriter<W>) -> Result<()> {
        zip.start_file(self.arc_name.clone(), zip::write::FileOptions::default())?;
        match self.var {
            DSymVar::FsFile(ref p) => {
                io::copy(&mut File::open(&p)?, &mut zip)?;
            }
            DSymVar::ZipFile(ref rc, idx) => {
                let rc = rc.clone();
                let mut opt_archive = rc.borrow_mut();
                if let Some(ref mut archive) = *opt_archive {
                    let mut af = archive.by_index(idx)?;
                    io::copy(&mut af, &mut zip)?;
                } else {
                    panic!("zip file went away");
                }
            }
        }
        Ok(())
    }
}

struct BatchIter {
    path: PathBuf,
    wd_iter: WalkDirIter,
    batch: Vec<DSymRef>,
    open_zip: Rc<RefCell<Option<zip::ZipArchive<fs::File>>>>,
    open_zip_index: usize,
}

impl BatchIter {
    pub fn new<P: AsRef<Path>>(path: P) -> BatchIter {
        BatchIter {
            path: path.as_ref().to_path_buf(),
            wd_iter: WalkDir::new(&path).into_iter(),
            batch: vec![],
            open_zip: Rc::new(RefCell::new(None)),
            open_zip_index: !0,
        }
    }
}

impl Iterator for BatchIter {
    type Item = Result<Vec<DSymRef>>;

    fn next(&mut self) -> Option<Result<Vec<DSymRef>>> {
        println!("  Creating DSym batch");
        let mut show_zip_continue = true;
        loop {
            if self.open_zip_index == !0 {
                *self.open_zip.borrow_mut() = None;
            }

            if self.open_zip_index != !0 {
                let mut archive_ptr = self.open_zip.borrow_mut();
                let mut archive = archive_ptr.as_mut().unwrap();
                if show_zip_continue {
                    println!("    Continue with zip archive");
                    show_zip_continue = false;
                }
                if self.open_zip_index >= archive.len() {
                    self.open_zip_index = !0;
                    if self.batch.len() != 0 {
                        break;
                    }
                } else {
                    let is_macho = {
                        let mut f = iter_try!(archive.by_index(self.open_zip_index));
                        is_macho_file(&mut f)
                    };
                    if is_macho {
                        let mut f = iter_try!(archive.by_index(self.open_zip_index));
                        let name = Path::new("DebugSymbols").join(f.name());
                        println!("      {}", name.display());
                        self.batch.push(DSymRef {
                            var: DSymVar::ZipFile(self.open_zip.clone(), self.open_zip_index),
                            arc_name: name.to_string_lossy().into_owned(),
                            checksum: iter_try!(get_sha1_checksum(&mut f)),
                        });
                        if self.batch.len() > BATCH_SIZE {
                            break;
                        }
                    }
                    self.open_zip_index += 1;
                }
            } else if let Some(dent_res) = self.wd_iter.next() {
                let dent = iter_try!(dent_res);
                let md = iter_try!(dent.metadata());
                if md.is_file() {
                    if is_macho_file(iter_try!(fs::File::open(&dent.path()))) {
                        let name = Path::new("DebugSymbols")
                            .join(dent.path().strip_prefix(&self.path).unwrap());
                        println!("    {}", name.display());
                        self.batch.push(DSymRef {
                            var: DSymVar::FsFile(dent.path().to_path_buf()),
                            arc_name: name.to_string_lossy().into_owned(),
                            checksum: iter_try!(get_sha1_checksum(
                                &mut iter_try!(fs::File::open(dent.path())))),
                        });
                        if self.batch.len() > BATCH_SIZE {
                            break;
                        }
                    } else if is_zip_file(iter_try!(fs::File::open(&dent.path()))) {
                        println!("    {} (zip archive)", dent.path().display());
                        show_zip_continue = false;
                        let f = iter_try!(fs::File::open(dent.path()));
                        *self.open_zip.borrow_mut() = Some(iter_try!(zip::ZipArchive::new(f)));
                        self.open_zip_index = 0;
                        // whenever we switch the zip we need to yield because we
                        // might have references to an earlier zip
                        if self.batch.len() > 0 {
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }

        if self.batch.len() == 0 {
            None
        } else {
            Some(Ok(mem::replace(&mut self.batch, vec![])))
        }
    }
}

fn find_missing_files(api: &mut Api,
                      refs: Vec<DSymRef>,
                      org: &str,
                      project: &str)
                      -> Result<Vec<DSymRef>> {
    let missing = {
        let checksums: Vec<_> = refs.iter().map(|ref x| x.checksum.as_str()).collect();
        api.find_missing_dsym_checksums(org, project, &checksums)?
    };
    let mut rv = vec![];
    for r in refs.into_iter() {
        if missing.contains(&r.checksum) {
            rv.push(r)
        }
    }
    Ok(rv)
}

fn zip_up(refs: &[DSymRef]) -> Result<TempFile> {
    println!("  Uploading a batch of missing files ...");
    let tf = TempFile::new()?;
    let mut zip = zip::ZipWriter::new(tf.open());
    for ref r in refs {
        println!("    {}", r.arc_name);
        r.add_to_archive(&mut zip)?;
    }
    Ok(tf)
}

fn upload_dsyms(api: &mut Api,
                refs: &[DSymRef],
                org: &str,
                project: &str)
                -> Result<Vec<DSymFile>> {
    let tf = zip_up(refs)?;
    Ok(api.upload_dsyms(org, project, tf.path())?)
}

fn get_paths_from_env() -> Result<Vec<PathBuf>> {
    let mut rv = vec![];
    if let Some(base_path) = env::var_os("DWARF_DSYM_FOLDER_PATH") {
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
    app.about("uploads debug symbols to a project")
        .arg(Arg::with_name("org")
            .value_name("ORG")
            .long("org")
            .short("o")
            .help("The organization slug"))
        .arg(Arg::with_name("project")
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .help("The project slug"))
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("The path to the debug symbols")
            .multiple(true)
            .index(1))
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Does not trigger reprocessing after upload"))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let paths = match matches.values_of("paths") {
        Some(paths) => paths.map(|x| PathBuf::from(x)).collect(),
        None => get_paths_from_env()?,
    };
    let (org, project) = config.get_org_and_project(matches)?;
    let mut api = Api::new(config);

    println!("Uploading symbols");
    if paths.len() == 0 {
        println!("Warning: no paths were provided.");
    }

    for path in paths {
        println!("Finding symbols in {}...", path.display());
        for batch_res in BatchIter::new(path) {
            println!("Detecting dsyms to upload");
            let missing = find_missing_files(&mut api, batch_res?, &org, &project)?;
            if missing.len() == 0 {
                println!("  No dsyms missing on server");
                continue;
            }
            println!("Detected {} missing dsym(s)", missing.len());
            let rv = upload_dsyms(&mut api, &missing, &org, &project)?;
            if rv.len() > 0 {
                println!("  Accepted debug symbols:");
                for df in rv {
                    println!("    {} ({}; {})", df.uuid, df.object_name, df.cpu_name);
                }
            }
        }
    }

    // If wanted trigger reprocessing
    if !matches.is_present("no_reprocessing") {
        if api.trigger_reprocessing(&org, &project)? {
            println!("Triggered reprocessing");
        } else {
            println!("Server does not support reprocessing. Not triggering.");
        }
    } else {
        println!("Skipped reprocessing.");
    }

    Ok(())
}
