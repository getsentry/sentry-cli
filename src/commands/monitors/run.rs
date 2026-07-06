#![expect(clippy::unwrap_used, reason = "contains legacy code which uses unwrap")]

use chrono_tz::Tz;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, Read as _, Write as _};
use std::process::{self, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::style;
use serde_json::Value;

use sentry::protocol::{
    Context, Event, Level, LogEntry, MonitorConfig, MonitorSchedule, TraceContext,
};
use sentry::Envelope;

use crate::api::envelopes_api::EnvelopesApi;
use crate::commands::send_event::send_raw_event;
use crate::utils::event::get_sdk_info;
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
        .arg(
            Arg::new("with_output")
                .long("with-output")
                .action(clap::ArgAction::SetTrue)
                .help("Capture command output (stdout and stderr combined)."),
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
                "{} could not invoke program '{}': {err}",
                style("error").red(),
                args[0]
            );
            (false, None)
        }
    };

    let elapsed = started.elapsed();
    (success, code, elapsed)
}

fn run_program_with_output(
    args: Vec<&String>,
    monitor_slug: &str,
) -> (bool, Option<i32>, Duration, Vec<u8>) {
    let started = Instant::now();
    let mut p = process::Command::new(args[0]);
    p.args(&args[1..]);
    p.env("SENTRY_MONITOR_SLUG", monitor_slug);
    p.stdout(Stdio::piped());
    p.stderr(Stdio::piped());

    let (success, code, output) = match p.spawn() {
        Ok(mut child) => {
            let combined = Arc::new(Mutex::new(Vec::new()));

            let mut stdout_pipe = child.stdout.take();
            let mut stderr_pipe = child.stderr.take();

            let out_handle = stdout_pipe.take().map(|mut pipe| {
                let combined = Arc::clone(&combined);
                thread::spawn(move || {
                    let mut buf = [0u8; 8192];
                    loop {
                        match pipe.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                let _ = io::stdout().write_all(&buf[..n]);
                                let _ = io::stdout().flush();
                                combined.lock().unwrap().extend_from_slice(&buf[..n]);
                            }
                            Err(_) => break,
                        }
                    }
                })
            });

            let err_handle = stderr_pipe.take().map(|mut pipe| {
                let combined = Arc::clone(&combined);
                thread::spawn(move || {
                    let mut buf = [0u8; 8192];
                    loop {
                        match pipe.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                let _ = io::stderr().write_all(&buf[..n]);
                                let _ = io::stderr().flush();
                                combined.lock().unwrap().extend_from_slice(&buf[..n]);
                            }
                            Err(_) => break,
                        }
                    }
                })
            });

            let (success, code) = match child.wait() {
                Ok(status) => (status.success(), status.code()),
                Err(err) => {
                    eprintln!(
                        "{} could not invoke program '{}': {err}",
                        style("error").red(),
                        args[0]
                    );
                    (false, None)
                }
            };

            if let Some(h) = out_handle {
                let _ = h.join();
            }
            if let Some(h) = err_handle {
                let _ = h.join();
            }

            let output = combined.lock().unwrap().clone();
            (success, code, output)
        }
        Err(err) => {
            eprintln!(
                "{} could not invoke program '{}': {err}",
                style("error").red(),
                args[0]
            );
            (false, None, Vec::new())
        }
    };

    let elapsed = started.elapsed();
    (success, code, elapsed, output)
}

#[allow(clippy::allow_attributes, clippy::too_many_arguments)]
fn send_checkin_with_trace(
    envelopes_api: &EnvelopesApi,
    check_in_id: Uuid,
    monitor_slug: &str,
    status: &str,
    environment: &str,
    trace_id: Uuid,
    duration: Option<f64>,
    monitor_config: Option<&MonitorConfig>,
) {
    let mut checkin = serde_json::json!({
        "check_in_id": check_in_id.to_string().replace('-', ""),
        "monitor_slug": monitor_slug,
        "status": status,
        "environment": environment,
        "contexts": {
            "trace": {
                "trace_id": trace_id.to_string().replace('-', "")
            }
        }
    });
    if let Some(d) = duration {
        checkin["duration"] = serde_json::json!(d);
    }
    if let Some(config) = monitor_config {
        if let Ok(config_json) = serde_json::to_value(config) {
            checkin["monitor_config"] = config_json;
        }
    }
    let payload = checkin.to_string();
    let envelope_id = Uuid::new_v4();
    let mut buf = Vec::new();
    use std::io::Write as _;
    let _ = writeln!(buf, r#"{{"event_id":"{envelope_id}"}}"#);
    let _ = writeln!(buf, r#"{{"type":"check_in","length":{}}}"#, payload.len());
    buf.extend_from_slice(payload.as_bytes());
    if let Ok(envelope) = Envelope::from_bytes_raw(buf) {
        if let Err(e) = envelopes_api.send_envelope(envelope) {
            log::error!("Failed to send check-in envelope: {e}");
        }
    }
}

fn execute_checkin(
    args: Vec<&String>,
    monitor_slug: &str,
    environment: &str,
    monitor_config: Option<MonitorConfig>,
    with_output: bool,
) -> Result<(bool, Option<i32>)> {
    let check_in_id = Uuid::new_v4();
    let trace_id = Uuid::new_v4();

    let envelopes_api = EnvelopesApi::try_new()?;

    send_checkin_with_trace(
        &envelopes_api,
        check_in_id,
        monitor_slug,
        "in_progress",
        environment,
        trace_id,
        None,
        monitor_config.as_ref(),
    );

    let (success, code, elapsed, output) = if with_output {
        let (s, c, e, o) = run_program_with_output(args, monitor_slug);
        (s, c, e, Some(o))
    } else {
        let (s, c, e) = run_program(args, monitor_slug);
        (s, c, e, None)
    };

    let status = if success { "ok" } else { "error" };
    let duration = Some(elapsed.as_secs_f64());

    send_checkin_with_trace(
        &envelopes_api,
        check_in_id,
        monitor_slug,
        status,
        environment,
        trace_id,
        duration,
        None,
    );

    if !success {
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output);

            let mut monitor_ctx = BTreeMap::new();
            monitor_ctx.insert("id".into(), Value::String(monitor_slug.to_owned()));
            monitor_ctx.insert("check_in_id".into(), Value::String(check_in_id.to_string()));

            let mut contexts = BTreeMap::new();
            contexts.insert("monitor".into(), Context::Other(monitor_ctx));

            let trace_bytes: [u8; 16] = *trace_id.as_bytes();
            let span_bytes: [u8; 8] = trace_bytes[..8].try_into().unwrap();
            let trace_ctx = TraceContext {
                trace_id: trace_bytes.into(),
                span_id: span_bytes.into(),
                ..TraceContext::default()
            };
            contexts.insert("trace".into(), Context::Trace(Box::new(trace_ctx)));

            let mut title = match code {
                Some(n) => format!("Cron failed (exit code {n})"),
                None => "Cron failed".to_string(),
            };
            title.push_str("\n\n");

            let mut message = output_str.into_owned();
            message.insert_str(0, &title);

            let event = Event {
                logentry: Some(LogEntry {
                    message,
                    params: vec![],
                }),
                transaction: Some(monitor_slug.to_owned()),
                level: Level::Error,
                environment: Some(Cow::Owned(environment.to_owned())),
                sdk: Some(get_sdk_info()),
                contexts,
                ..Event::default()
            };

            match send_raw_event(event) {
                Ok(event_id) => log::info!("Monitor output event sent: {event_id}"),
                Err(e) => log::error!("Failed to send monitor output event: {e}"),
            }
        }
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
    let with_output = matches.get_flag("with_output");

    let (success, code) =
        execute_checkin(args, monitor_slug, environment, monitor_config, with_output)?;

    if !success {
        return Err(QuietExit(code.unwrap_or(1)).into());
    }

    Ok(())
}
