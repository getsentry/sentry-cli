use super::derive_parser::{SentryCLI, SentryCLICommand};
use crate::config::Config;
use crate::utils::event;
use crate::utils::metrics::payload::MetricPayload;
use anyhow::Context;
use anyhow::Result;
use clap::{command, Args, Subcommand};
use clap::{ArgMatches, Command, Parser};
use log::debug;
use sentry::protocol::EnvelopeItem;
use sentry::Envelope;
use subcommands::SendMetricSubcommand;

pub mod subcommands;

#[derive(Args)]
pub(super) struct SendMetricArgs {
    #[command(subcommand)]
    pub(super) subcommand: SendMetricSubcommand,
}

pub(super) fn make_command(command: Command) -> Command {
    SendMetricSubcommand::augment_subcommands(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::SendMetric(SendMetricArgs { subcommand }) = SentryCLI::parse().command;
    let mut envelope = Envelope::new();
    let payload = Result::<MetricPayload>::from(subcommand)?;
    envelope.add_item(EnvelopeItem::Statsd(payload.to_bytes()));
    let dsn = Config::current().get_dsn().ok().context(
        "DSN not found. \
    See: https://docs.sentry.io/product/crons/getting-started/cli/#configuration",
    )?;
    event::with_sentry_client(dsn, |c| c.send_envelope(envelope));
    debug!("Metric payload sent: {}", (payload.to_string()?));
    Ok(())
}
