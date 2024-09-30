use chrono_tz::Tz;
use std::process;
use std::time::{Duration, Instant};
use uuid::Uuid;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::style;

use sentry::protocol::{MonitorCheckIn, MonitorCheckInStatus, MonitorConfig, MonitorSchedule};

use crate::api::envelopes_api::EnvelopesApi;
use crate::utils::system::QuietExit;
use crate::utils::value_parsers::auth_token_parser;

pub fn make_command(command: Command) -> Command {
    command
        .about("Wraps a command")
        .arg(
            Arg::new("monitor_slug")
                .value_name("monitor-slug")
                .help("The monitor slug.")
                .required(true),
        )
        .arg(
            Arg::new("environment")
                .short('e')
                .long("environment")
                .default_value("production")
                .help("Specify the environment of the monitor."),
        )
        .arg(
            Arg::new("args")
                .value_name("ARGS")
                .required(true)
                .num_args(1..)
                .last(true),
        )
        .arg(Arg::new("schedule").short('s').long("schedule").help(
            "Configure the cron monitor with the given schedule (crontab format). \
             Enclose the schedule in quotes to ensure your command line environment \
             parses the argument correctly.",
        ))
        .arg(
            Arg::new("checkin_margin")
                .long("check-in-margin")
                .value_parser(clap::value_parser!(u64).range(1..))
                .requires("schedule")
                .help(
                    "The allowed margin of minutes after the expected check-in time that the \
                     monitor will not be considered missed for. Requires --schedule.",
                ),
        )
        .arg(
            Arg::new("max_runtime")
                .long("max-runtime")
                .value_parser(clap::value_parser!(u64).range(1..))
                .requires("schedule")
                .help(
                    "The allowed duration in minutes that the monitor may be in progress for \
                     before being considered failed due to timeout. Requires --schedule.",
                ),
        )
        .arg(
            Arg::new("timezone")
                .long("timezone")
                .value_parser(|value: &str| {
                    value.parse::<Tz>().map_err(|err| {
                        err + "\n\tSee here for a list of valid timezone strings: \
                            https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List"
                    })
                })
                .requires("schedule")
                .help(
                    "A tz database string (e.g. \"Europe/Vienna\") representing the monitor's \
                    execution schedule's timezone. Requires --schedule.",
                ),
        )
        .arg(
            Arg::new("failure_issue_threshold")
                .long("failure-issue-threshold")
                .value_parser(clap::value_parser!(u64).range(1..))
                .requires("schedule")
                .help(
                    "The number of consecutive missed or error check-ins that trigger an \
                     issue. Requires --schedule.",
                ),
        )
        .arg(
            Arg::new("recovery_threshold")
                .long("recovery-threshold")
                .value_parser(clap::value_parser!(u64).range(1..))
                .requires("schedule")
                .help(
                    "The number of consecutive successful check-ins that resolve an \
                     issue. Requires --schedule.",
                ),
        )
        // Hide auth token from --help output
        .arg(
            Arg::new("auth_token")
                .long("auth-token")
                .value_parser(auth_token_parser)
                .hide(true),
        )
}

fn run_program(args: Vec<&String>, monitor_slug: &str) -> (bool, Option<i32>, Duration) {
    let started = Instant::now();
    let mut p = process::Command::new(args[0]);
    p.args(&args[1..]);
    p.env("SENTRY_MONITOR_SLUG", monitor_slug);

    let (success, code) = match p.status() {
        Ok(status) => (status.success(), status.code()),
        Err(err) => {
            eprintln!(
                "{} could not invoke program '{}': {}",
                style("error").red(),
                args[0],
                err
            );
            (false, None)
        }
    };

    let elapsed = started.elapsed();
    (success, code, elapsed)
}

fn execute_checkin(
    args: Vec<&String>,
    monitor_slug: &str,
    environment: &str,
    monitor_config: Option<MonitorConfig>,
) -> Result<(bool, Option<i32>)> {
    let check_in_id = Uuid::new_v4();

    let open_checkin = MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.to_string(),
        status: MonitorCheckInStatus::InProgress,
        duration: None,
        environment: Some(environment.to_string()),
        monitor_config,
    };

    let envelopes_api = EnvelopesApi::try_new()?;

    if let Err(e) = envelopes_api.send_envelope(open_checkin) {
        log::error!("Failed to send in-progress check-in envelope: {e}");
        log::info!("Continuing to run program...");
    }

    let (success, code, elapsed) = run_program(args, monitor_slug);

    let status = if success {
        MonitorCheckInStatus::Ok
    } else {
        MonitorCheckInStatus::Error
    };

    let duration = Some(elapsed.as_secs_f64());

    let close_checkin = MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.to_string(),
        status,
        duration,
        environment: Some(environment.to_string()),
        monitor_config: None,
    };

    if let Err(e) = envelopes_api.send_envelope(close_checkin) {
        log::error!("Failed to send final check-in envelope: {e}");
        log::info!("Continuing to exit with program's exit code...");
    }

    Ok((success, code))
}

fn parse_monitor_config_args(matches: &ArgMatches) -> Result<Option<MonitorConfig>> {
    let Some(schedule) = matches.get_one::<String>("schedule") else {
        return Ok(None);
    };
    let schedule = MonitorSchedule::from_crontab(schedule)?;
    Ok(Some(MonitorConfig {
        schedule,
        checkin_margin: matches.get_one("checkin_margin").copied(),
        max_runtime: matches.get_one("max_runtime").copied(),
        timezone: matches.get_one("timezone").map(Tz::to_string),
        failure_issue_threshold: matches.get_one("failure_issue_threshold").copied(),
        recovery_threshold: matches.get_one("recovery_threshold").copied(),
    }))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let args: Vec<_> = matches.get_many::<String>("args").unwrap().collect();
    let monitor_slug = matches.get_one::<String>("monitor_slug").unwrap();
    let environment = matches.get_one::<String>("environment").unwrap();
    let monitor_config = parse_monitor_config_args(matches)?;

    let (success, code) = execute_checkin(args, monitor_slug, environment, monitor_config)?;

    if !success {
        return Err(QuietExit(code.unwrap_or(1)).into());
    }

    Ok(())
}
