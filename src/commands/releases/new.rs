use anyhow::Result;
use chrono::Utc;
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, NewRelease};
use crate::config::Config;
use crate::utils::args::ArgExt;

pub fn make_command(command: Command) -> Command {
    command
        .about("Create a new release.")
        .allow_hyphen_values(true)
        .version_arg()
        .arg(
            Arg::new("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."),
        )
        .arg(
            Arg::new("finalize")
                .long("finalize")
                .help("Immediately finalize the release. (sets it to released)"),
        )
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(Arg::new("ref").long("ref").hide(true))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();

    api.new_release(
        &config.get_org(matches)?,
        &NewRelease {
            version: version.to_owned(),
            projects: config.get_projects(matches)?,
            url: matches.get_one::<String>("url").cloned(),
            date_started: Some(Utc::now()),
            date_released: if matches.contains_id("finalize") {
                Some(Utc::now())
            } else {
                None
            },
        },
    )?;

    println!("Created release {version}");
    Ok(())
}
