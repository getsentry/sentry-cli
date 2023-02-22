use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::{get_timestamp, ArgExt};

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
                .value_parser(get_timestamp)
                .value_name("TIMESTAMP")
                .help("Set the release start date."),
        )
        .arg(
            Arg::new("released")
                .long("released")
                .value_parser(get_timestamp)
                .value_name("TIMESTAMP")
                .help("Set the release time. [defaults to the current time]"),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();

    api.update_release(
        &config.get_org(matches)?,
        version,
        &UpdatedRelease {
            projects: config.get_projects(matches).ok(),
            url: matches.get_one::<String>("url").cloned(),
            date_started: matches.get_one::<DateTime<Utc>>("started").copied(),
            date_released: Some(
                matches
                    .get_one::<DateTime<Utc>>("released")
                    .map_or_else(Utc::now, |v| *v),
            ),
            ..Default::default()
        },
    )?;

    println!("Finalized release {version}");
    Ok(())
}
