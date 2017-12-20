//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "1024"]

use std::env;

extern crate anylog;
extern crate app_dirs;
extern crate backtrace;
#[cfg(not(windows))]
#[macro_use]
extern crate chan;
#[cfg(not(windows))]
extern crate chan_signal;
extern crate chardet;
extern crate chrono;
extern crate clap;
extern crate console;
extern crate curl;
extern crate dotenv;
extern crate elementtree;
extern crate encoding;
#[macro_use]
extern crate error_chain;
extern crate git2;
extern crate glob;
extern crate hostname;
extern crate humansize;
#[macro_use]
extern crate if_chain;
extern crate ignore;
extern crate indicatif;
extern crate ini;
extern crate itertools;
extern crate java_properties;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
#[cfg(target_os = "macos")]
extern crate mac_process_info;
extern crate memmap;
extern crate might_be_minified;
extern crate open;
#[cfg(not(windows))]
extern crate openssl_probe;
#[cfg(target_os = "macos")]
extern crate osascript;
extern crate plist;
extern crate prettytable;
extern crate regex;
extern crate runas;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate sha1;
extern crate sourcemap;
extern crate symbolic_common;
extern crate symbolic_debuginfo;
extern crate symbolic_proguard;
#[cfg(not(windows))]
extern crate uname;
#[cfg(target_os = "macos")]
extern crate unix_daemonize;
extern crate url;
extern crate username;
extern crate uuid;
extern crate walkdir;
extern crate which;
extern crate zip;

mod macros;

pub mod api;
pub mod commands;
pub mod config;
pub mod constants;
pub mod errors;
pub mod event;
pub mod prelude;
pub mod utils;

/// Executes the command line application and exits the process.
pub fn main() {
    if env::var("SENTRY_LOAD_DOTENV")
        .map(|x| x.as_str() == "1")
        .unwrap_or(true)
    {
        dotenv::dotenv().ok();
    }
    utils::init_backtrace();
    utils::run_or_interrupt(commands::main);
}
