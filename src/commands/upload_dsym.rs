//! Implements a command for uploading dsym files.

use std::io;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::mem;
use std::ffi::OsStr;

use clap::{App, Arg, ArgMatches};
use walkdir::{WalkDir, Iter as WalkDirIter};
use zip;

use prelude::*;
use api::{Api, DSymFile};
use utils::{TempFile, get_sha1_checksum};
use macho::is_macho_file;
use config::Config;

const BATCH_SIZE : usize = 15;


struct LocalFile {
    path: PathBuf,
    arc_name: String,
    checksum: String,
}

struct BatchIter {
    path: PathBuf,
    wd_iter: WalkDirIter,
    batch: Vec<LocalFile>,
}

impl BatchIter {
    pub fn new<P: AsRef<Path>>(path: P) -> BatchIter {
        BatchIter {
            path: path.as_ref().to_path_buf(),
            wd_iter: WalkDir::new(&path).into_iter(),
            batch: vec![],
        }
    }
}

impl Iterator for BatchIter {
    type Item = Result<Vec<LocalFile>>;

    fn next(&mut self) -> Option<Result<Vec<LocalFile>>> {
        loop {
            if let Some(dent_res) = self.wd_iter.next() {
                let dent = iter_try!(dent_res);
                let md = iter_try!(dent.metadata());
                if md.is_file() && is_macho_file(dent.path()) {
                    let name = Path::new("DebugSymbols")
                        .join(dent.path().strip_prefix(&self.path).unwrap());
                    println!("  {}", name.display());
                    self.batch.push(LocalFile {
                        path: dent.path().to_path_buf(),
                        arc_name: name.to_string_lossy().into_owned(),
                        checksum: iter_try!(get_sha1_checksum(dent.path())),
                    });
                    if self.batch.len() > BATCH_SIZE {
                        break;
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

fn find_missing_files(api: &mut Api, files: Vec<LocalFile>, org: &str, project: &str)
    -> Result<Vec<LocalFile>>
{
    let missing = {
        let checksums : Vec<_> = files.iter().map(|ref x| x.checksum.as_str()).collect();
        api.find_missing_dsym_checksums(org, project, &checksums)?
    };
    let mut rv = vec![];
    for file in files.into_iter() {
        if missing.contains(&file.checksum) {
            rv.push(file)
        }
    }
    Ok(rv)
}

fn zip_up(files: &[LocalFile]) -> Result<TempFile> {
    println!("  Uploading a batch of missing files ...");
    let tf = TempFile::new()?;
    let mut zip = zip::ZipWriter::new(tf.open());
    for ref file in files {
        println!("    {}", file.arc_name);
        zip.start_file(file.arc_name.clone(),
            zip::CompressionMethod::Deflated)?;
        io::copy(&mut File::open(file.path.clone())?, &mut zip)?;
    }
    Ok(tf)
}

fn upload_dsyms(api: &mut Api, files: &[LocalFile],
                org: &str, project: &str) -> Result<Vec<DSymFile>> {
    let tf = zip_up(files)?;
    Ok(api.upload_dsyms(org, project, tf.path())?)
}

fn get_paths_from_env() -> Result<Vec<PathBuf>> {
    let mut rv = vec![];
    if let Some(base_path) = env::var_os("DWARF_DSYM_FOLDER_PATH") {
        for entry in fs::read_dir(base_path)? {
            let entry = entry?;
            if entry.path().extension() == Some(OsStr::new("dSYM")) &&
                fs::metadata(entry.path())?.is_dir() {
                rv.push(entry.path().to_path_buf());
            }
        }
    }
    Ok(rv)
}


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("uploads debug symbols to a project")
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
            let missing = find_missing_files(&mut api, batch_res?, &org, &project)?;
            if missing.len() == 0 {
                continue;
            }
            println!("Detected missing files");
            let rv = upload_dsyms(&mut api, &missing, &org, &project)?;
            if rv.len() > 0 {
                println!("  Accepted debug symbols:");
                for df in rv {
                    println!("    {} ({}; {})", df.uuid, df.object_name, df.cpu_name);
                }
            }
        }
    }

    Ok(())
}
