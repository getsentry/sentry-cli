use std::path::PathBuf;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use glob::{glob_with, MatchOptions};
use log::{debug, warn};
use sentry::types::Dsn;
use sentry::Envelope;

use crate::config::Config;
use crate::utils::event::with_sentry_client;

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
}

fn send_raw_envelope(envelope: Envelope, dsn: Dsn) {
    debug!("{:?}", envelope);
    with_sentry_client(dsn, |c| c.send_envelope(envelope));
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let dsn = config.get_dsn()?;

    let path = matches.value_of("path").unwrap();

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
        let envelope: Envelope = Envelope::from_path(p)?;
        send_raw_envelope(envelope, dsn.clone());
        println!("Envelope from file {} dispatched", p.display());
    }

    Ok(())
}
