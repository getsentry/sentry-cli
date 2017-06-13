use std::io;
use std::string;

use ini::ini;
use clap;
use glob;
use serde_json;
use url;
use walkdir;
use zip;
use plist;
use sourcemap;
use elementtree;
use git2;
use mach_object;
use proguard;

use api;

error_chain! {
    errors {
        QuietExit(code: i32) {
            description("sentry-cli quit")
        }
        NoMacho {
            description("not a mach-o file")
        }
    }

    links {
        Proguard(proguard::Error, proguard::ErrorKind);
    }

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
        Git(git2::Error);
        MachO(mach_object::Error);
        GlobPattern(glob::PatternError);
        Xml(elementtree::Error);
    }
}
