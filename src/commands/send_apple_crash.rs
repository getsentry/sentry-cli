use anyhow::{Context as _, Result};
use clap::{Arg, ArgAction, ArgMatches, Args, Command};
use log::info;
use sentry::types::Uuid;
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use crate::api::envelopes_api::EnvelopesApi;
use crate::utils::apple_crash::parse_ips_crash_report;
use crate::utils::args::validate_distribution;

/// Arguments for send-apple-crash command
#[derive(Args)]
#[command(about = "Send Apple crash reports to Sentry")]
#[command(long_about = "Send Apple crash reports (.ips) to Sentry.\n\n\
    This command parses Apple crash report files in .ips (JSON) format \
    and sends them to Sentry as error events. Sentry will automatically \
    symbolicate the crash reports if matching debug symbols (dSYMs) have \
    been uploaded.\n\n\
    Due to network errors, rate limits or sampling the event is not guaranteed to \
    actually arrive. Check debug output for transmission errors by passing --log-level=\
    debug or setting SENTRY_LOG_LEVEL=debug.")]
pub(super) struct SendAppleCrashArgs {
    #[arg(value_name = "PATH")]
    #[arg(help = "Path to one or more .ips files to send as crash events")]
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    #[arg(short = 'r', long = "release")]
    #[arg(help = "Optional release identifier to associate with the crash")]
    release: Option<String>,

    #[arg(short = 'E', long = "env")]
    #[arg(help = "Optional environment name (e.g., production, staging)")]
    environment: Option<String>,

    #[arg(short = 'd', long = "dist")]
    #[arg(value_parser = validate_distribution)]
    #[arg(help = "Optional distribution identifier")]
    dist: Option<String>,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Send Apple crash reports to Sentry.")
        .long_about(
            "Send Apple crash reports (.ips) to Sentry.{n}{n}\
             This command parses Apple crash report files in .ips (JSON) format \
             and sends them to Sentry as error events. Sentry will automatically \
             symbolicate the crash reports if matching debug symbols (dSYMs) have \
             been uploaded.{n}{n}\
             Due to network errors, rate limits or sampling the event is not guaranteed to \
             actually arrive. Check debug output for transmission errors by passing --log-level=\
             debug or setting `SENTRY_LOG_LEVEL=debug`.",
        )
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .action(ArgAction::Append)
                .required(true)
                .help("Path to one or more .ips files to send as crash events"),
        )
        .arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .short('r')
                .help("Optional release identifier to associate with the crash"),
        )
        .arg(
            Arg::new("environment")
                .value_name("ENVIRONMENT")
                .long("env")
                .short('E')
                .help("Optional environment name (e.g., production, staging)"),
        )
        .arg(
            Arg::new("dist")
                .value_name("DISTRIBUTION")
                .long("dist")
                .short('d')
                .value_parser(validate_distribution)
                .help("Optional distribution identifier"),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let paths: Vec<PathBuf> = matches
        .get_many::<String>("paths")
        .map(|vals| vals.map(PathBuf::from).collect())
        .unwrap_or_default();

    let release = matches.get_one::<String>("release");
    let environment = matches.get_one::<String>("environment");
    let dist = matches.get_one::<String>("dist");

    // Process each crash file path
    for path in paths.iter() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read crash file: {}", path.display()))?;

        // Parse the .ips file into a Sentry event
        let mut event = parse_ips_crash_report(&content)
            .with_context(|| format!("Failed to parse crash file: {}", path.display()))?;

        // Override with CLI arguments if provided
        if let Some(release) = release {
            event.release = Some(Cow::Owned(release.clone()));
        }
        if let Some(environment) = environment {
            event.environment = Some(Cow::Owned(environment.clone()));
        }
        if let Some(dist) = dist {
            event.dist = Some(Cow::Owned(dist.clone()));
        }

        // Send the event
        let event_id = send_raw_event(event)?;
        println!(
            "Crash from file {} dispatched: {event_id}",
            path.display()
        );
        info!("Crash event {event_id} sent successfully");
    }

    Ok(())
}

/// Send a Sentry event via envelope
fn send_raw_event(event: sentry::protocol::Event<'static>) -> Result<Uuid> {
    use crate::constants::USER_AGENT;
    use sentry::{apply_defaults, Client, ClientOptions};

    let client = Client::from_config(apply_defaults(ClientOptions {
        user_agent: USER_AGENT.into(),
        ..Default::default()
    }));
    let event = client
        .prepare_event(event, None)
        .ok_or_else(|| anyhow::anyhow!("Event dropped during preparation"))?;
    let event_id = event.event_id;
    EnvelopesApi::try_new()?.send_envelope(event)?;
    Ok(event_id)
}
