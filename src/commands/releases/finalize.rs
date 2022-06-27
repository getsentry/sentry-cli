use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::{get_timestamp, validate_timestamp, ArgExt};

pub fn make_command(command: Command) -> Command {
    command
        .about("Mark a release as finalized and released.")
        .allow_hyphen_values(true)
        .version_arg()
        .arg(
            Arg::new("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."),
        )
        .arg(
            Arg::new("started")
                .long("started")
                .validator(validate_timestamp)
                .value_name("TIMESTAMP")
                .help("Set the release start date."),
        )
        .arg(
            Arg::new("released")
                .long("released")
                .validator(validate_timestamp)
                .value_name("TIMESTAMP")
                .help("Set the release time. [defaults to the current time]"),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    fn get_date(value: Option<&str>, now_default: bool) -> Result<Option<DateTime<Utc>>> {
        match value {
            None => Ok(if now_default { Some(Utc::now()) } else { None }),
            Some(value) => Ok(Some(get_timestamp(value)?)),
        }
    }

    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();

    api.update_release(
        &config.get_org(matches)?,
        version,
        &UpdatedRelease {
            projects: config.get_projects(matches).ok(),
            url: matches.value_of("url").map(str::to_owned),
            date_started: get_date(matches.value_of("started"), false)?,
            date_released: get_date(matches.value_of("released"), true)?,
            ..Default::default()
        },
    )?;

    println!("Finalized release {}", version);
    Ok(())
}
