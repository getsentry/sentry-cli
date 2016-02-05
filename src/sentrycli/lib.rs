extern crate argparse;
extern crate hyper;
extern crate mime;
extern crate multipart;
extern crate rustc_serialize;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate zip;

// what we export
pub use version::get_version;
pub use error::CliError;
pub type CliResult<T> = Result<T, CliError>;

mod macros;

mod version;
mod commands;
mod error;
mod utils;

pub fn main() -> ! {
    commands::main();
}
