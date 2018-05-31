//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "128"]

extern crate anylog;
extern crate app_dirs;
extern crate backtrace;
extern crate brotli2;
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
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate flate2;
extern crate git2;
extern crate glob;
extern crate hostname;
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
extern crate might_be_minified;
extern crate open;
#[cfg(not(windows))]
extern crate openssl_probe;
#[cfg(target_os = "macos")]
extern crate osascript;
extern crate parking_lot;
extern crate plist;
extern crate prettytable;
extern crate regex;
extern crate runas;
extern crate scoped_threadpool;
#[macro_use]
extern crate sentry;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate sha1;
extern crate sourcemap;
extern crate symbolic;
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
pub mod event;
pub mod utils;

/// Executes the command line application and exits the process.
pub fn main() {
    utils::system::run_or_interrupt(commands::main);
}
