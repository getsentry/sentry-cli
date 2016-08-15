use std::io;
use std::string;

use ini::ini;
use clap;
use serde_json;
use url;
use walkdir;
use zip;
use sourcemap;

use api;

error_chain! {
    foreign_links {
        io::Error, IoError;
        zip::result::ZipError, ZipError;
        walkdir::Error, WalkDirError;
        url::ParseError, UrlError;
        serde_json::Error, JsonError;
        string::FromUtf8Error, FromUtf8Error;
        ini::Error, IniError;
        sourcemap::Error, SourceMapError;
        clap::Error, ClapError;

        api::Error, ApiError;
    }
}
