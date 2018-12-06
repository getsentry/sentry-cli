//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "128"]

#[macro_use]
#[cfg(not(windows))]
extern crate chan;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate if_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate sentry;

pub mod api;
pub mod commands;
pub mod config;
pub mod constants;
pub mod utils;

/// Executes the command line application and exits the process.
pub fn main() {
    utils::system::run_or_interrupt(commands::main);
}
