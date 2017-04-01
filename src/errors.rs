use std::io;
use std::string;

use crates::ini::ini;
use crates::clap;
use crates::serde_json;
use crates::url;
use crates::walkdir;
use crates::zip;
use crates::plist;
use crates::sourcemap;
use crates::elementtree;

use api;

error_chain! {
    foreign_links {
        Io(io::Error);
        Zip(zip::result::ZipError);
        WalkDir(walkdir::Error);
        UrlParse(url::ParseError);
        Json(serde_json::Error);
        FromUtf8(string::FromUtf8Error);
        Ini(ini::Error);
        SourceMap(sourcemap::Error);
        Clap(clap::Error);
        PList(plist::Error);
        Api(api::Error);
        Xml(elementtree::ParseError);
    }
}
