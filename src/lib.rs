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
extern crate term;
extern crate might_be_minified;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

mod macros;

pub mod prelude;
pub mod api;
pub mod commands;
pub mod event;
pub mod errors;
pub mod config;
pub mod utils;
pub mod macho;
pub mod sourcemaputils;
pub mod constants;


#[cfg(not(windows))]
fn real_main() {
    if let Some(signal) = utils::run_or_interrupt(commands::main) {
        use chan_signal::Signal;
        if signal == Signal::INT {
            println!("Interrupted!");
        }
    }
}

#[cfg(windows)]
fn real_main() {
    commands::main();
}

/// Executes the command line application and exits the process.
pub fn main() {
    real_main();
}
