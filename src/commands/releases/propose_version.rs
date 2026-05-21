use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::allow_xcode_infoplist_preprocessing_arg;
use crate::utils::releases::detect_release_name;

pub fn make_command(command: Command) -> Command {
    command
        .about("Propose a version name for a new release.")
        .arg(allow_xcode_infoplist_preprocessing_arg())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    println!(
        "{}",
        detect_release_name(matches.get_flag("allow_xcode_infoplist_preprocessing"))?
    );
    Ok(())
}
