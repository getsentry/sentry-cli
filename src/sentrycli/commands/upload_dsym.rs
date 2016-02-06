use std::io;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use hyper::method::Method;
use multipart::client::Multipart;
use serde_json;
use walkdir::WalkDir;
use zip;

use super::super::CliResult;
use super::super::utils::TempFile;
use super::Config;


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

fn make_archive<P: AsRef<Path>>(path: P) -> CliResult<TempFile> {
    let tf = try!(TempFile::new());
    let file = try!(File::create(&tf.path()));
    let mut zip = zip::ZipWriter::new(file);

    let it = WalkDir::new(&path)
        .max_depth(5)
        .into_iter();

    let arc_base = Path::new("DebugSymbols.dSYM");

    for dent_res in it {
        let dent = try!(dent_res);
        let md = try!(dent.metadata());
        if md.is_file() {
            let name = arc_base.join(dent.path().strip_prefix(&path).unwrap());
            try!(zip.start_file(
                name.to_string_lossy().into_owned(),
                zip::CompressionMethod::Deflated));
            let mut f = try!(File::open(dent.path()));
            try!(io::copy(&mut f, &mut zip));
        }
    }

    try!(zip.finish());
    
    Ok(tf)
}

fn upload_dsyms(tf: &TempFile, config: &Config,
                org: &str, project: &str) -> CliResult<Vec<DSymFile>> {
    let req = try!(config.api_request(
        Method::Post, &format!("/projects/{}/{}/files/dsyms/", org, project)));
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
             .help("The organization slug")
             .required(true)
             .index(1))
        .arg(Arg::with_name("project")
             .value_name("PROJECT")
             .help("The project slug")
             .required(true)
             .index(2))
        .arg(Arg::with_name("path")
             .value_name("PATH")
             .help("The path to the debug symbols")
             .required(true)
             .index(3))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    let path = matches.value_of("path").unwrap();
    let org = matches.value_of("org").unwrap();
    let project = matches.value_of("project").unwrap();

    let tf = try!(make_archive(path));
    let rv = try!(upload_dsyms(&tf, config, org, project));

    if rv.len() == 0 {
        println!("Server did not accept any debug symbols.");
    } else {
        println!("Accepted debug symbols:");
        for df in rv {
            println!("  {} ({}; {})", df.uuid, df.object_name, df.cpu_name);
        }
    }

    Ok(())
}
