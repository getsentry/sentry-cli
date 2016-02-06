#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate clap;
extern crate hyper;
extern crate mime;
extern crate multipart;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate zip;
extern crate serde;
extern crate serde_json;

// what we export
pub use error::CliError;
pub type CliResult<T> = Result<T, CliError>;

mod macros;

mod commands;
mod error;
mod utils;

pub fn main() -> ! {
    commands::main();
}
