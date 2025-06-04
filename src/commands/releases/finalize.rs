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
        .version_arg(false)
        .arg(
            Arg::new("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."),
        )
        .arg(
            Arg::new("started")
                .long("started")
                .hide(true)
                .value_parser(get_timestamp)
                .value_name("TIMESTAMP")
                .help("[DEPRECATED] This value is ignored."),
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
    #[expect(clippy::unwrap_used, reason = "legacy code")]
    let version = matches.get_one::<String>("version").unwrap();

    if matches.get_one::<DateTime<Utc>>("started").is_some() {
        log::warn!(
            "The --started flag is deprecated. Its value is ignored, \
            and the argument will be completely removed in a future version."
        );
    }

    api.authenticated()?.update_release(
        &config.get_org(matches)?,
        version,
        &UpdatedRelease {
            projects: config.get_projects(matches).ok(),
            url: matches.get_one::<String>("url").cloned(),
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
