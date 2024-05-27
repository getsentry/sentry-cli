pub mod common_args;

mod distribution;
mod gauge;
mod increment;
mod set;

use self::common_args::FloatValueMetricArgs;
use self::increment::IncrementMetricArgs;
use self::set::SetMetricArgs;
use super::derive_parser::{SentryCLI, SentryCLICommand};
use anyhow::Result;
use clap::{command, Args, Subcommand};
use clap::{ArgMatches, Command, Parser};

#[derive(Args)]
pub(super) struct SendMetricArgs {
    #[command(subcommand)]
    subcommand: SendMetricSubcommand,
}

#[derive(Subcommand)]
#[command(about = "Send a metric to Sentry.")]
#[command(long_about = "Send a metric event to Sentry.{n}{n}\
This command will validate input parameters and attempt to send a metric to \
Sentry. Due to network errors and rate limits, the metric is not guaranteed to \
arrive. Check the debug output for transmission errors by passing --log-level=\
debug or setting `SENTRY_LOG_LEVEL=debug`.")]
enum SendMetricSubcommand {
    #[command(about = "Increment a counter metric")]
    Increment(IncrementMetricArgs),
    #[command(about = "Update a distribution metric with the provided value")]
    Distribution(FloatValueMetricArgs),
    #[command(about = "Update a gauge metric with the provided value")]
    Gauge(FloatValueMetricArgs),
    #[command(about = "Update a set metric with the provided value")]
    Set(SetMetricArgs),
}

pub(super) fn make_command(command: Command) -> Command {
    SendMetricSubcommand::augment_subcommands(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    // When adding a new subcommand to the derive_parser SentryCLI, replace the line below with the following:
    // let subcommand = match SentryCLI::parse().command {
    //     SentryCLICommand::SendMetric(SendMetricArgs { subcommand }) => subcommand,
    //     _ => panic!("expected send-metric subcommand"),
    // };
    let SentryCLICommand::SendMetric(SendMetricArgs { subcommand }) = SentryCLI::parse().command;
    match subcommand {
        SendMetricSubcommand::Increment(args) => increment::execute(args),
        SendMetricSubcommand::Distribution(args) => distribution::execute(args),
        SendMetricSubcommand::Gauge(args) => gauge::execute(args),
        SendMetricSubcommand::Set(args) => set::execute(args),
    }
}
