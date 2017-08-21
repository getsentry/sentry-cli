//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "1024"]

#[cfg(not(windows))]
#[macro_use]
extern crate chan;
#[cfg(not(windows))]
extern crate chan_signal;
extern crate curl;
extern crate clap;
extern crate ini;
extern crate backtrace;
extern crate itertools;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate which;
extern crate zip;
extern crate sha1;
extern crate sourcemap;
extern crate open;
extern crate runas;
extern crate plist;
extern crate proguard;
extern crate elementtree;
extern crate might_be_minified;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate if_chain;
extern crate chrono;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate console;
extern crate indicatif;
#[cfg(target_os="macos")]
extern crate osascript;
#[cfg(target_os="macos")]
extern crate unix_daemonize;
extern crate dotenv;
#[cfg(not(windows))]
extern crate openssl_probe;
extern crate prettytable;
extern crate git2;
extern crate humansize;
extern crate java_properties;
extern crate mach_object;
extern crate memmap;
extern crate glob;
extern crate libc;
#[cfg(target_os="macos")]
extern crate mac_process_info;
extern crate app_dirs;
extern crate uchardet;
extern crate encoding;

mod macros;

pub mod prelude;
pub mod api;
pub mod commands;
pub mod event;
pub mod errors;
pub mod config;
pub mod utils;
pub mod constants;

/// Executes the command line application and exits the process.
pub fn main() {
    dotenv::dotenv().ok();
    utils::init_backtrace();
    utils::run_or_interrupt(commands::main);
}
