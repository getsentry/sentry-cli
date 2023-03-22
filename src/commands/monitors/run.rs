use log::warn;
use std::process;
use std::time::Instant;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;

use crate::api::{Api, CreateMonitorCheckIn, MonitorCheckinStatus, UpdateMonitorCheckIn};
use crate::config::Config;
use crate::utils::system::{print_error, QuietExit};

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
            Arg::new("allow_failure")
                .short('f')
                .long("allow-failure")
                .action(ArgAction::SetTrue)
                .help("Run provided command even when Sentry reports an error."),
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
    let api = Api::current();
    let config = Config::current();
    let dsn = config.get_dsn().ok();

    // Token based auth is deprecated, prefer DSN style auth for monitor checkins.
    // Using token based auth *DOES NOT WORK* when using slugs.
    if dsn.is_none() {
        warn!("Token auth is deprecated for cron monitor checkins. Please use DSN auth.");
    }

    let monitor_slug = matches.get_one::<String>("monitor_slug").unwrap();

    let allow_failure = matches.get_flag("allow_failure");
    let args: Vec<_> = matches.get_many::<String>("args").unwrap().collect();

    let monitor_checkin = api.create_monitor_checkin(
        dsn.clone(),
        monitor_slug,
        &CreateMonitorCheckIn {
            status: MonitorCheckinStatus::InProgress,
        },
    );

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

    match monitor_checkin {
        Ok(checkin) => {
            api.update_monitor_checkin(
                dsn,
                monitor_slug,
                &checkin.id,
                &UpdateMonitorCheckIn {
                    status: Some(if success {
                        MonitorCheckinStatus::Ok
                    } else {
                        MonitorCheckinStatus::Error
                    }),
                    duration: Some({
                        let elapsed = started.elapsed();
                        elapsed.as_secs() * 1000 + u64::from(elapsed.subsec_millis())
                    }),
                },
            )
            .ok();
        }
        Err(e) => {
            if allow_failure {
                print_error(&anyhow::Error::from(e));
            } else {
                return Err(e.into());
            }
        }
    }

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
