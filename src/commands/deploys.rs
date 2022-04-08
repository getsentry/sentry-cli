use anyhow::Result;
use chrono::{Duration, Utc};
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, Deploy};
use crate::config::Config;
use crate::utils::args::{get_timestamp, validate_int, validate_timestamp, ArgExt};
use crate::utils::formatting::{HumanDuration, Table};

pub fn make_command(command: Command) -> Command {
    command
        .about("Manage deployments for Sentry releases.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        .release_arg()
        .subcommand(
            Command::new("new")
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
                        .validator(validate_timestamp)
                        .help("Optional unix timestamp when the deployment started."),
                )
                .arg(
                    Arg::new("finished")
                        .long("finished")
                        .value_name("TIMESTAMP")
                        .validator(validate_timestamp)
                        .help("Optional unix timestamp when the deployment finished."),
                )
                .arg(
                    Arg::new("time")
                        .long("time")
                        .short('t')
                        .value_name("SECONDS")
                        .validator(validate_int)
                        .help(
                            "Optional deployment duration in seconds.{n}\
                            This can be specified alternatively to `--started` and `--finished`.",
                        ),
                ),
        )
        .subcommand(
            Command::new("list")
                // Backward compatibility with `releases deploys <VERSION>` commands.
                .arg(Arg::new("version").long("version").hide(true))
                .about("List all deployments of a release."),
        )
}

fn execute_deploys_new(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = config.get_release_with_legacy_fallback(matches)?;
    let mut deploy = Deploy {
        env: matches.value_of("env").unwrap().to_string(),
        name: matches.value_of("name").map(str::to_owned),
        url: matches.value_of("url").map(str::to_owned),
        ..Default::default()
    };

    if let Some(value) = matches.value_of("time") {
        let finished = Utc::now();
        deploy.finished = Some(finished);
        deploy.started = Some(finished - Duration::seconds(value.parse().unwrap()));
    } else {
        if let Some(finished_str) = matches.value_of("finished") {
            deploy.finished = Some(get_timestamp(finished_str)?);
        } else {
            deploy.finished = Some(Utc::now());
        }
        if let Some(started_str) = matches.value_of("started") {
            deploy.started = Some(get_timestamp(started_str)?);
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

fn execute_deploys_list(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = config.get_release_with_legacy_fallback(matches)?;
    let mut table = Table::new();
    table
        .title_row()
        .add("Environment")
        .add("Name")
        .add("Finished");

    for deploy in api.list_deploys(&config.get_org(matches)?, &version)? {
        table
            .add_row()
            .add(&deploy.env)
            .add(deploy.name())
            .add(HumanDuration(
                Utc::now().signed_duration_since(deploy.finished.unwrap()),
            ));
    }

    if table.is_empty() {
        println!("No deploys found");
    } else {
        table.print();
    }

    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_deploys_new(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_deploys_list(sub_matches);
    }
    unreachable!();
}
