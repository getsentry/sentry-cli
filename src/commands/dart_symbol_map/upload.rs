use anyhow::Result;
use clap::{ArgMatches, Command};

// Reuse the existing implementation
pub fn make_command(command: Command) -> Command {
    crate::commands::upload_dart_symbol_map::make_command(command)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    crate::commands::upload_dart_symbol_map::execute(matches)
}


