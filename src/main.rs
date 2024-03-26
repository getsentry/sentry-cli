//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binary that links against this library.

mod api;
mod commands;
mod config;
mod constants;
mod utils;

/// Executes the command line application and exits the process.
pub fn main() {
    commands::main()
}
