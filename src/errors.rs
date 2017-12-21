use std::io;
use std::string;

use clap;
use elementtree;
use git2;
use glob;
use ignore;
use ini::ini;
use plist;
use serde_json;
use sourcemap;
use symbolic_common;
use url;
use walkdir;
use zip;

use api;

error_chain! {
    errors {
        QuietExit(code: i32) {
            description("sentry-cli quit")
        }
    }

    foreign_links {
        Api(api::Error);
        Clap(clap::Error);
        FromUtf8(string::FromUtf8Error);
        Git(git2::Error);
        GlobPattern(glob::PatternError);
        Ignore(ignore::Error);
        Ini(ini::Error);
        Io(io::Error);
        Json(serde_json::Error);
        PList(plist::Error);
        SourceMap(sourcemap::Error);
        Symbolic(symbolic_common::Error);
        UrlParse(url::ParseError);
        WalkDir(walkdir::Error);
        Xml(elementtree::Error);
        Zip(zip::result::ZipError);
    }
}
