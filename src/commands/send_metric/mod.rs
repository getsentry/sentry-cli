use self::common_args::FloatValueMetricArgs;
use self::increment::IncrementArgs;
use self::set::SetArgs;
use super::derive_parser::{SentryCLI, SentryCLICommand};
use anyhow::Result;
use clap::{command, Args, Subcommand};
use clap::{ArgMatches, Command, Parser};

pub mod common_args;
mod distribution;
mod gauge;
mod increment;
mod set;

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
arrive.")]
enum SendMetricSubcommand {
    #[command(about = "Send an increment to a counter metric")]
    Increment(IncrementArgs),
    #[command(about = "Send a value to a distribution metric")]
    Distribution(FloatValueMetricArgs),
    #[command(about = "Send a value to a gauge metric")]
    Gauge(FloatValueMetricArgs),
    #[command(about = "Send a value to a set metric")]
    Set(SetArgs),
}

pub(super) fn make_command(command: Command) -> Command {
    SendMetricSubcommand::augment_subcommands(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::SendMetric(SendMetricArgs { subcommand }) = SentryCLI::parse().command;
    match subcommand {
        SendMetricSubcommand::Increment(args) => increment::execute(args),
        SendMetricSubcommand::Distribution(args) => distribution::execute(args),
        SendMetricSubcommand::Gauge(args) => gauge::execute(args),
        SendMetricSubcommand::Set(args) => set::execute(args),
    }
}
