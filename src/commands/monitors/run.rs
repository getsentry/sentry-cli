use std::process;
use std::time::Instant;
use uuid::Uuid;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;

use sentry::protocol::{MonitorCheckIn, MonitorCheckInStatus};

use crate::config::Config;
use crate::utils::event::with_sentry_client;
use crate::utils::system::QuietExit;

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
                .default_value("production")
                .help("Specify the environment of the monitor."),
        )
        .arg(
            Arg::new("allow_failure")
                .short('f')
                .long("allow-failure")
                .action(ArgAction::SetTrue)
                .help("Run provided command even when Sentry reports an error.")
                .hide(true),
        )
        .arg(
            Arg::new("args")
                .value_name("ARGS")
                .required(true)
                .num_args(1..)
                .last(true),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let dsn = config
        .get_dsn()
        .expect("DSN auth is required for monitor execution");
    let monitor_slug = matches.get_one::<String>("monitor_slug").unwrap();
    let environment = matches.get_one::<String>("environment").unwrap();
    let args: Vec<_> = matches.get_many::<String>("args").unwrap().collect();

    let check_in_id = Uuid::new_v4();

    let open_checkin = MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.to_string(),
        status: MonitorCheckInStatus::InProgress,
        duration: None,
        environment: Some(environment.to_string()),
        monitor_config: None,
    };

    with_sentry_client(dsn.clone(), |c| c.send_envelope(open_checkin.into()));

    let started = Instant::now();
    let mut p = process::Command::new(args[0]);
    p.args(&args[1..]);
    p.env("SENTRY_MONITOR_SLUG", monitor_slug.clone());

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

    let status = if success {
        MonitorCheckInStatus::Ok
    } else {
        MonitorCheckInStatus::Error
    };

    let duration = Some(started.elapsed().as_secs_f64());

    let close_checkin = MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.to_string(),
        status,
        duration,
        environment: Some(environment.to_string()),
        monitor_config: None,
    };

    with_sentry_client(dsn, |c| c.send_envelope(close_checkin.into()));

    if !success {
        return Err(QuietExit(code.unwrap_or(1)).into());
    }

    Ok(())
}
