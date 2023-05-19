use log::warn;
use std::process;
use std::time::Instant;
use uuid::Uuid;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
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
            Arg::new("args")
                .value_name("ARGS")
                .required(true)
                .num_args(1..)
                .last(true),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();

    let dsn = match config.get_dsn() {
        Ok(dsn) => dsn,
        Err(_) => {
            warn!("DSN auth is required for monitor execution");
            return Err(QuietExit(1).into());
        }
    };

    let monitor_slug = matches
        .get_one::<String>("monitor_slug")
        .unwrap()
        .to_owned();
    let environment = matches.get_one::<String>("environment").unwrap().to_owned();
    let args: Vec<_> = matches.get_many::<String>("args").unwrap().collect();

    let check_in_id = Uuid::new_v4();

    let open_checkin = MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.clone(),
        status: MonitorCheckInStatus::InProgress,
        duration: None,
        environment: Some(environment.clone()),
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
        monitor_slug,
        status,
        duration,
        environment: Some(environment),
        monitor_config: None,
    };

    with_sentry_client(dsn, |c| c.send_envelope(close_checkin.into()));

    if !success {
        if let Some(code) = code {
            Err(QuietExit(code).into())
        } else {
            Err(QuietExit(1).into())
        }
    } else {
        Ok(())
    }
}
