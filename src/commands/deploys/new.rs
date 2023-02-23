use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, Deploy};
use crate::config::Config;
use crate::utils::args::get_timestamp;

pub fn make_command(command: Command) -> Command {
    command
        .about("Creates a new release deployment.")
        // Backward compatibility with `releases deploys <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
        .arg(
            Arg::new("env")
                .long("env")
                .short('e')
                .value_name("ENV")
                .required(true)
                .help(
                    "Set the environment for this release.{n}This argument is required.  \
                            Values that make sense here would be 'production' or 'staging'.",
                ),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .short('n')
                .value_name("NAME")
                .help("Optional human readable name for this deployment."),
        )
        .arg(
            Arg::new("url")
                .long("url")
                .short('u')
                .value_name("URL")
                .help("Optional URL that points to the deployment."),
        )
        .arg(
            Arg::new("started")
                .long("started")
                .value_name("TIMESTAMP")
                .value_parser(get_timestamp)
                .help("Optional unix timestamp when the deployment started."),
        )
        .arg(
            Arg::new("finished")
                .long("finished")
                .value_name("TIMESTAMP")
                .value_parser(get_timestamp)
                .help("Optional unix timestamp when the deployment finished."),
        )
        .arg(
            Arg::new("time")
                .long("time")
                .short('t')
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(i64))
                .help(
                    "Optional deployment duration in seconds.{n}\
                            This can be specified alternatively to `--started` and `--finished`.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = config.get_release_with_legacy_fallback(matches)?;
    let mut deploy = Deploy {
        env: matches.get_one::<String>("env").unwrap().to_string(),
        name: matches.get_one::<String>("name").cloned(),
        url: matches.get_one::<String>("url").cloned(),
        ..Default::default()
    };

    if let Some(value) = matches.get_one::<i64>("time") {
        let finished = Utc::now();
        deploy.finished = Some(finished);
        deploy.started = Some(finished - Duration::seconds(*value));
    } else {
        if let Some(finished) = matches.get_one::<DateTime<Utc>>("finished") {
            deploy.finished = Some(*finished);
        } else {
            deploy.finished = Some(Utc::now());
        }
        if let Some(started) = matches.get_one::<DateTime<Utc>>("started") {
            deploy.started = Some(*started);
        }
    }

    let org = config.get_org(matches)?;
    let created_deploy = api.create_deploy(&org, &version, &deploy)?;

    println!(
        "Created new deploy {} for '{}'",
        created_deploy.name(),
        created_deploy.env
    );

    Ok(())
}
