use std::io;
use std::io::Read;
use std::path::Path;
use std::fs::File;

use argparse;
use hyper::method::Method;
use multipart::client::Multipart;
use walkdir::WalkDir;
use zip;

use super::super::CliResult;
use super::super::utils::TempFile;
use super::{Config, parse_args_or_abort};


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

pub fn execute(args: Vec<String>, config: &Config) -> CliResult<()> {
    let mut path = "".to_owned();
    let mut org = "".to_owned();
    let mut project = "".to_owned();

    {
        let mut ap = argparse::ArgumentParser::new();
        ap.set_description("Uploads dsym symbols.");
        ap.refer(&mut org)
            .required()
            .add_argument("org", argparse::Store,
                          "the organization slug");
        ap.refer(&mut project)
            .required()
            .add_argument("project", argparse::Store,
                          "the project slug");
        ap.refer(&mut path)
            .required()
            .add_argument("path", argparse::Store,
                          "path to the debug symbol bundle");
        try!(parse_args_or_abort(&ap, args));
    }

    let tf = try!(make_archive(path));

    let req = try!(config.api_request(
        Method::Post, &format!("/projects/{}/{}/files/dsyms/", org, project)));
    let mut mp = try!(Multipart::from_request_sized(req));
    mp.write_file("file", &tf.path());

    let mut resp = try!(mp.send());
    println!("{:?}", resp);
    let mut s = String::new();
    resp.read_to_string(&mut s).unwrap();
    println!("{}", s);

    Ok(())
}
