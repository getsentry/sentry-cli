use std::io;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;
use std::process::{Command, Stdio};

use clap::{App, Arg, ArgMatches};
use hyper::method::Method;
use multipart::client::Multipart;
use serde_json;
use walkdir::WalkDir;
use zip;
use which::which;

use super::super::CliResult;
use super::super::utils::TempFile;
use super::Config;
use super::super::macho::is_macho_file;

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

fn make_archive<P: AsRef<Path>>(path: P, use_dsymutil: bool) -> CliResult<TempFile> {
    let tf = try!(TempFile::new());
    let file = try!(File::create(&tf.path()));
    let mut zip = zip::ZipWriter::new(file);

    let it = WalkDir::new(&path).into_iter();

    let arc_base = Path::new("DebugSymbols");

    for dent_res in it {
        let dent = try!(dent_res);
        let md = try!(dent.metadata());
        if md.is_file() && is_macho_file(dent.path()) {
            let name = arc_base.join(dent.path().strip_prefix(&path).unwrap());
            try!(zip.start_file(
                name.to_string_lossy().into_owned(),
                zip::CompressionMethod::Deflated));
            println!("  {}", name.display());
            if use_dsymutil {
                let sf = try!(invoke_dsymutil(dent.path()));
                let mut f = try!(File::open(sf.path()));
                try!(io::copy(&mut f, &mut zip));
            } else {
                let mut f = try!(File::open(dent.path()));
                try!(io::copy(&mut f, &mut zip));
            }
        }
    }

    try!(zip.finish());
    
    Ok(tf)
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
    println!("Creating archive from {}...", path);
    let tf = try!(make_archive(path, use_dsymutil));

    println!("Uploading archive ...");
    let rv = try!(upload_dsyms(&tf, config, &target));

    if rv.len() == 0 {
        fail!("Server did not accept any debug symbols.");
    } else {
        println!("");
        println!("Accepted debug symbols:");
        for df in rv {
            println!("  {} ({}; {})", df.uuid, df.object_name, df.cpu_name);
        }
    }
    Ok(())
}
