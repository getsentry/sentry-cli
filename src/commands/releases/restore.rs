use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::{Api, ReleaseStatus, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::ArgExt;

pub fn make_command(command: Command) -> Command {
    command
        .about("Restore a release.")
        .allow_hyphen_values(true)
        .version_arg()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();

    let info_rv = api.update_release(
        &config.get_org(matches)?,
        version,
        &UpdatedRelease {
            projects: Some(vec![]),
            version: Some(version.into()),
            status: Some(ReleaseStatus::Open),
            ..Default::default()
        },
    )?;

    println!("Restored release {}", info_rv.version);
    Ok(())
}
