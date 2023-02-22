use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;

pub fn make_command(command: Command) -> Command {
    command
        .about("Delete a release.")
        .allow_hyphen_values(true)
        .version_arg()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();
    let project = config.get_project(matches).ok();

    if api.delete_release(&config.get_org(matches)?, project.as_deref(), version)? {
        println!("Deleted release {version}!");
    } else {
        println!("Did nothing. Release with this version ({version}) does not exist.");
    }

    Ok(())
}
