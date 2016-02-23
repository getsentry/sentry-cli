use std::io;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::HashMap;
use std::process::{Command, Stdio};

use clap::{App, Arg, ArgMatches};
use hyper::method::Method;
use multipart::client::Multipart;
use serde_json;
use walkdir::{WalkDir, Iter as WalkDirIter};
use zip;
use which::which;

use super::super::CliResult;
use super::super::utils::TempFile;
use super::Config;
use super::super::macho::is_macho_file;

const BATCH_SIZE : u32 = 10;

enum UploadTarget {
    Global,
    Project {
        org: String,
        project: String
    }
}

impl UploadTarget {

    pub fn get_api_path(&self) -> String {
        match *self {
            UploadTarget::Global => "/system/global-dsyms/".to_owned(),
            UploadTarget::Project { ref org, ref project } => {
                format!("/projects/{}/{}/files/dsyms/", org, project)
            }
        }
    }
}


// XXX: when serde 0.7 lands we can remove the unused ones here.
// Currently we need them as it does otherwise error out on parsing :(
#[derive(Debug, Deserialize)]
struct DSymFile {
    id: String,
    sha1: String,
    uuid: String,
    size: i64,
    #[serde(rename="objectName")]
    object_name: String,
    #[serde(rename="symbolType")]
    symbol_type: String,
    headers: HashMap<String, String>,
    #[serde(rename="dateCreated")]
    date_created: String,
    #[serde(rename="cpuName")]
    cpu_name: String,
}

fn invoke_dsymutil(path: &Path) -> CliResult<TempFile> {
    let tf = try!(TempFile::new());
    let out = try!(Command::new("dsymutil")
        .arg("-o")
        .arg(&tf.path())
        .arg("--flat")
        .arg(&path)
        .stderr(Stdio::null())
        .output());
    if out.status.success() {
        Ok(tf)
    } else {
        fail!("dsymutil failed to extract symbols");
    }
}

struct BatchIterTarget {
    pub tf: TempFile,
    pub zip: zip::ZipWriter<File>,
    pub item_count: u32,
}

struct BatchIter {
    path: PathBuf,
    wd_iter: WalkDirIter,
    use_dsymutil: bool,
    target: Option<BatchIterTarget>,
}

impl BatchIter {
    pub fn new<P: AsRef<Path>>(path: P, use_dsymutil: bool) -> BatchIter {
        BatchIter {
            path: path.as_ref().to_path_buf(),
            wd_iter: WalkDir::new(&path).into_iter(),
            use_dsymutil: use_dsymutil,
            target: None,
        }
    }

    fn ensure_target(&mut self) -> CliResult<()> {
        match self.target {
            Some(_) => Ok(()),
            None => {
                println!("Creating new batch:");
                let tf = try!(TempFile::new());
                let f = try!(File::create(tf.path()));
                self.target = Some(BatchIterTarget {
                    tf: tf,
                    zip: zip::ZipWriter::new(f),
                    item_count: 0,
                });
                Ok(())
            }
        }
    }
}

impl Iterator for BatchIter {
    type Item = CliResult<TempFile>;

    fn next(&mut self) -> Option<CliResult<TempFile>> {
        loop {
            if let Some(dent_res) = self.wd_iter.next() {
                let dent = iter_try!(dent_res);
                let md = iter_try!(dent.metadata());
                if md.is_file() && is_macho_file(dent.path()) {
                    iter_try!(self.ensure_target());
                    {
                        let target = &mut self.target.as_mut().unwrap();
                        let arc_base = Path::new("DebugSymbols");
                        let name = arc_base.join(dent.path().strip_prefix(&self.path).unwrap());
                        iter_try!(target.zip.start_file(
                            name.to_string_lossy().into_owned(),
                            zip::CompressionMethod::Deflated));
                        println!("  {}", name.display());
                        if self.use_dsymutil {
                            let sf = iter_try!(invoke_dsymutil(dent.path()));
                            let mut f = iter_try!(File::open(sf.path()));
                            iter_try!(io::copy(&mut f, &mut target.zip));
                        } else {
                            let mut f = iter_try!(File::open(dent.path()));
                            iter_try!(io::copy(&mut f, &mut target.zip));
                        }
                        target.item_count += 1;
                    }
                    if self.target.as_ref().unwrap().item_count > BATCH_SIZE {
                        return Some(Ok(self.target.take().unwrap().tf));
                    }
                }
            } else {
                break;
            }
        }
        self.target.take().map(|val| Ok(val.tf))
    }
}

fn upload_dsyms(tf: &TempFile, config: &Config,
                target: &UploadTarget) -> CliResult<Vec<DSymFile>> {
    let req = try!(config.api_request(Method::Post, &target.get_api_path()));
    let mut mp = try!(Multipart::from_request_sized(req));
    mp.write_file("file", &tf.path());
    let mut resp = try!(mp.send());
    Ok(try!(serde_json::from_reader(&mut resp)))
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
        .arg(Arg::with_name("global")
             .long("global")
             .short("g")
             .help("Uploads the dsyms globally. This can only be done \
                    with super admin access for the Sentry installation"))
        .arg(Arg::with_name("use_dsymutil")
             .long("use-dsymutil")
             .help("Invoke dsymutil on encountered macho binaries to extract \
                    the symbols before uploading.  This requires the dsymutil \
                    binary to be available."))
        .arg(Arg::with_name("path")
             .value_name("PATH")
             .help("The path to the debug symbols")
             .required(true)
             .index(1))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    let path = matches.value_of("path").unwrap();
    let target = if matches.is_present("global") {
        UploadTarget::Global
    } else {
        if !matches.is_present("org") || !matches.is_present("project") {
            fail!("For non global uploads both organization and project are required");
        }
        UploadTarget::Project {
            org: matches.value_of("org").unwrap().to_owned(),
            project: matches.value_of("project").unwrap().to_owned(),
        }
    };
    let use_dsymutil = matches.is_present("use_dsymutil");

    if use_dsymutil {
        if let Err(_) = which("dsymutil") {
            fail!("dsymutil not installed but required for operation.");
        } else {
            println!("Extracting symbols with dsymutil.");
        }
    }

    println!("Creating archives from {}...", path);

    let iter = BatchIter::new(path, use_dsymutil);
    for tf_res in iter {
        let tf = try!(tf_res);
        println!("Uploading archive ...");
        let rv = try!(upload_dsyms(&tf, config, &target));
        if rv.len() == 0 {
            fail!("Server did not accept any debug symbols.");
        } else {
            println!("Accepted debug symbols:");
            for df in rv {
                println!("  {} ({}; {})", df.uuid, df.object_name, df.cpu_name);
            }
        }
    }

    Ok(())
}
