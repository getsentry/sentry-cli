#![feature(custom_derive, plugin, question_mark, alloc_system)]
#![plugin(serde_macros)]

extern crate alloc_system;
#[cfg(not(windows))]
#[macro_use]
extern crate chan;
#[cfg(not(windows))]
extern crate chan_signal;
extern crate curl;
extern crate clap;
extern crate ini;
extern crate itertools;
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
#[macro_use]
extern crate log;

// what we export
pub use error::{CliError, CliResult};

mod macros;

mod api;
mod commands;
mod event;
mod error;
mod config;
mod utils;
mod macho;
mod sourcemaps;
mod constants;


#[cfg(not(windows))]
pub fn main() {
    if let Some(signal) = utils::run_or_interrupt(commands::main) {
        use chan_signal::Signal;
        if signal == Signal::INT {
            println!("Interrupted!");
        }
    }
}

#[cfg(windows)]
pub fn main() {
    commands::main();
}
