use log::warn;
use std::process;
use std::time::Instant;
use uuid::Uuid;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use console::style;

use sentry::protocol::{MonitorCheckIn, MonitorCheckInStatus};

use crate::api::{Api, ApiCreateMonitorCheckIn, ApiMonitorCheckInStatus, ApiUpdateMonitorCheckIn};
use crate::config::Config;
use crate::utils::event::with_sentry_client;
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
    let api = Api::current();
    let config = Config::current();
    let dsn = config.get_dsn().ok();

    // Token based auth is deprecated, prefer DSN style auth for monitor checkins.
    // Using token based auth *DOES NOT WORK* when using slugs.
    if dsn.is_none() {
        warn!("Token auth is deprecated for cron monitor checkins and will be removed in the next major version.");
        warn!("Please use DSN auth.");
    }

    let monitor_slug = matches.get_one::<String>("monitor_slug").unwrap();
    let environment = matches.get_one::<String>("environment").unwrap();
    let allow_failure = matches.get_flag("allow_failure");

    let args: Vec<_> = matches.get_many::<String>("args").unwrap().collect();

    let exec_pgrm = || {
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
    };

    let (success, code) = match dsn {
        // Use envelope API when dsn is provided. This is the prefered way to create check-ins, the
        // legacy API will be removed in the next major CLI version.
        Some(dsn) => {
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

            let (success, code, elapsed) = exec_pgrm();

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

            with_sentry_client(dsn, |c| c.send_envelope(close_checkin.into()));

            (success, code)
        }
        // Use legacy API when DSN is not provided
        None => {
            let monitor_checkin = api.create_monitor_checkin(
                monitor_slug,
                &ApiCreateMonitorCheckIn {
                    status: ApiMonitorCheckInStatus::InProgress,
                    environment: environment.to_string(),
                },
            );

            let (success, code, elapsed) = exec_pgrm();

            match monitor_checkin {
                Ok(checkin) => {
                    let status = if success {
                        ApiMonitorCheckInStatus::Ok
                    } else {
                        ApiMonitorCheckInStatus::Error
                    };

                    let duration =
                        Some(elapsed.as_secs() * 1000 + u64::from(elapsed.subsec_millis()));

                    api.update_monitor_checkin(
                        monitor_slug,
                        &checkin.id,
                        &ApiUpdateMonitorCheckIn {
                            status: Some(status),
                            duration,
                            environment: Some(environment.to_string()),
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

            (success, code)
        }
    };

    if !success {
        return Err(QuietExit(code.unwrap_or(1)).into());
    }

    Ok(())
}
