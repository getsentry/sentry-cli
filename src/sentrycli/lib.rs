#![feature(custom_derive, plugin, question_mark)]
#![plugin(serde_macros)]

#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate clap;
extern crate hyper;
extern crate ini;
#[macro_use]
extern crate mime;
extern crate mime_guess;
extern crate multipart;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate which;
extern crate zip;
extern crate sha1;

// what we export
pub use error::{CliError, CliResult};

use chan_signal::Signal;

mod macros;

mod commands;
mod error;
mod utils;
mod macho;


pub fn main() {
    if let Some(signal) = utils::run_or_interrupt(commands::main) {
        if signal == Signal::INT {
            println!("Interrupted!");
        }
    }
}
