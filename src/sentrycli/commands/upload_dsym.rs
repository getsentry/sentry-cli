use std::io;
use std::path::{Path, PathBuf};
use std::fs::File;

use clap::{App, Arg, ArgMatches};
use hyper::method::Method;
use mime;
use multipart::client::Multipart;
use serde_json;
use walkdir::{WalkDir, Iter as WalkDirIter};
use zip;

use CliResult;
use utils::{TempFile, get_org_and_project};
use macho::is_macho_file;
use commands::Config;

const BATCH_SIZE : u32 = 15;


#[derive(Debug, Deserialize)]
struct DSymFile {
    uuid: String,
    #[serde(rename="objectName")]
    object_name: String,
    #[serde(rename="cpuName")]
    cpu_name: String,
}

struct BatchIterBatch {
    pub tf: TempFile,
    pub zip: zip::ZipWriter<File>,
    pub item_count: u32,
}

struct BatchIter {
    path: PathBuf,
    wd_iter: WalkDirIter,
    batch: Option<BatchIterBatch>,
    batch_count: u32,
}

impl BatchIter {
    pub fn new<P: AsRef<Path>>(path: P) -> BatchIter {
        BatchIter {
            path: path.as_ref().to_path_buf(),
            wd_iter: WalkDir::new(&path).into_iter(),
            batch: None,
            batch_count: 0
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
                    // if we don't have a batch yet, create a new one.  We fill
                    // up to a fixed number of items into that batch before it
                    // is being returned from the iterator.
                    let batch = match self.batch {
                        None => {
                            self.batch_count += 1;
                            println!("Creating batch #{}:", self.batch_count);
                            let tf = iter_try!(TempFile::new());
                            let zip = zip::ZipWriter::new(tf.open());
                            self.batch = Some(BatchIterBatch {
                                tf: tf,
                                zip: zip,
                                item_count: 0,
                            });
                            self.batch.as_mut().unwrap()
                        },
                        Some(ref mut val) => val,
                    };

                    let name = Path::new("DebugSymbols")
                        .join(dent.path().strip_prefix(&self.path).unwrap());
                    iter_try!(batch.zip.start_file(
                        name.to_string_lossy().into_owned(),
                        zip::CompressionMethod::Deflated));
                    println!("  {}", name.display());
                    iter_try!(io::copy(&mut iter_try!(File::open(dent.path())),
                                       &mut batch.zip));
                    batch.item_count += 1;
                    if batch.item_count > BATCH_SIZE {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        self.batch.take().map(|val| Ok(val.tf))
    }
}

fn upload_dsyms(tf: &TempFile, config: &Config,
                api_path: &str) -> CliResult<Vec<DSymFile>> {
    let req = try!(config.prepare_api_request(Method::Post, api_path));
    let mut mp = try!(Multipart::from_request_sized(req));
    mp.write_stream("file", &mut tf.open(), Some("archive.zip"),
        "application/zip".parse::<mime::Mime>().ok());
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
        .arg(Arg::with_name("path")
             .value_name("PATH")
             .help("The path to the debug symbols")
             .required(true)
             .index(1))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    let path = matches.value_of("path").unwrap();
    let api_path = if matches.is_present("global") {
        "/system/global-dsyms/".to_owned()
    } else {
        let (org, project) = try!(get_org_and_project(matches));
        format!("/projects/{}/{}/files/dsyms/", org, project)
    };

    println!("Uploading symbols from {}...", path);

    for tf_res in BatchIter::new(path) {
        let tf = try!(tf_res);
        println!("Uploading archive ...");
        let rv = try!(upload_dsyms(&tf, config, &api_path));
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
