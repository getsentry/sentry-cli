use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::{Api, ReleaseStatus, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::ArgExt as _;

pub fn make_command(command: Command) -> Command {
    command
        .about("Restore a release.")
        .allow_hyphen_values(true)
        .version_arg(false)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    #[expect(clippy::unwrap_used, reason = "legacy code")]
    let version = matches.get_one::<String>("version").unwrap();

    let info_rv = api.authenticated()?.update_release(
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
