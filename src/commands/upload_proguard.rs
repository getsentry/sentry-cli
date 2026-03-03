use anyhow::Result;
use clap::{ArgMatches, Command};

pub fn make_command(command: Command) -> Command {
    // Retained as a top-level command for backward compatibility.
    crate::commands::proguard::upload::make_command(command)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    crate::commands::proguard::upload::execute(matches)
}
