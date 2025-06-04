#![expect(clippy::unwrap_used, reason = "contains legacy code which uses unwrap")]

use std::path::PathBuf;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use glob::{glob_with, MatchOptions};
use log::warn;
use sentry::Envelope;

use crate::api::envelopes_api::EnvelopesApi;

pub fn make_command(command: Command) -> Command {
    command
        .about("Send a stored envelope to Sentry.")
        .long_about(
            "Send a stored envelope to Sentry.{n}{n}\
             This command will validate and attempt to send an envelope to Sentry. \
             Due to network errors, rate limits or sampling the envelope is not guaranteed to \
             actually arrive. Check debug output for transmission errors by passing --log-level=\
             debug or setting `SENTRY_LOG_LEVEL=debug`.",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path or glob to the file(s) in envelope format to send as envelope(s)."),
        )
        .arg(
            Arg::new("raw")
                .long("raw")
                .action(ArgAction::SetTrue)
                .help("Send envelopes without attempting to parse their contents."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let raw = matches.get_flag("raw");

    let path = matches.get_one::<String>("path").unwrap();

    let collected_paths: Vec<PathBuf> = glob_with(path, MatchOptions::new())
        .unwrap()
        .flatten()
        .collect();

    if collected_paths.is_empty() {
        warn!("Did not match any envelope files for pattern: {}", path);
        return Ok(());
    }

    for path in collected_paths {
        let p = path.as_path();
        let envelope: Envelope = if raw {
            Envelope::from_path_raw(p)
        } else {
            Envelope::from_path(p)
        }?;
        EnvelopesApi::try_new()?.send_envelope(envelope)?;
        println!("Envelope from file {} dispatched", p.display());
    }

    Ok(())
}
