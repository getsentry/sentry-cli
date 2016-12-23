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
        IoError(io::Error);
        ZipError(zip::result::ZipError);
        WalkDirError(walkdir::Error);
        UrlError(url::ParseError);
        JsonError(serde_json::Error);
        FromUtf8Error(string::FromUtf8Error);
        IniError(ini::Error);
        SourceMapError(sourcemap::Error);
        ClapError(clap::Error);

        ApiError(api::Error);
    }
}
