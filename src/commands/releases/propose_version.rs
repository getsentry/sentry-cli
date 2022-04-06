use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::releases::detect_release_name;

pub fn make_command(command: Command) -> Command {
    command.about("Propose a version name for a new release.")
}

pub fn execute(_matches: &ArgMatches) -> Result<()> {
    println!("{}", detect_release_name()?);
    Ok(())
}
