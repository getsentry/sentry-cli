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

const DEPRECATION_MESSAGE: &str = "DEPRECATION NOTICE: \
    The send-metric commands are deprecated and will be \
    removed in the next major release. \
    Sentry will reject all metrics sent after October 7, 2024. Learn more: \
    https://sentry.zendesk.com/hc/en-us/articles/26369339769883-Upcoming-API-Changes-to-Metrics";

const INCREMENT_ABOUT: &str = "Increment a counter metric";
const DISTRIBUTION_ABOUT: &str = "Update a distribution metric with the provided value";
const GAUGE_ABOUT: &str = "Update a gauge metric with the provided value";
const SET_ABOUT: &str = "Update a set metric with the provided value";

#[derive(Args)]
pub(super) struct SendMetricArgs {
    #[command(subcommand)]
    subcommand: SendMetricSubcommand,
}

#[derive(Subcommand)]
#[command(about = "[DEPRECATED] Send a metric to Sentry.")]
#[command(long_about = format!("{DEPRECATION_MESSAGE}{{n}}{{n}}\
Send a metric event to Sentry.{{n}}{{n}}\
This command will validate input parameters and attempt to send a metric to \
Sentry. Due to network errors and rate limits, the metric is not guaranteed to \
arrive. Check the debug output for transmission errors by passing --log-level=\
debug or setting `SENTRY_LOG_LEVEL=debug`."))]
#[command(hide=true)]
enum SendMetricSubcommand {
    #[command(about = format!("[DEPRECATED] {INCREMENT_ABOUT}"))]
    #[command(long_about = format!("{DEPRECATION_MESSAGE}{{n}}{{n}}{INCREMENT_ABOUT}"))]
    Increment(IncrementMetricArgs),
    #[command(about = format!("[DEPRECATED] {DISTRIBUTION_ABOUT}"))]
    #[command(long_about = format!("{DEPRECATION_MESSAGE}{{n}}{{n}}{DISTRIBUTION_ABOUT}"))]
    Distribution(FloatValueMetricArgs),
    #[command(about = format!("[DEPRECATED] {GAUGE_ABOUT}"))]
    #[command(long_about = format!("{DEPRECATION_MESSAGE}{{n}}{{n}}{GAUGE_ABOUT}"))]
    Gauge(FloatValueMetricArgs),
    #[command(about = format!("[DEPRECATED] {SET_ABOUT}"))]
    #[command(long_about = format!("{DEPRECATION_MESSAGE}{{n}}{{n}}{SET_ABOUT}"))]
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

    log::warn!("{DEPRECATION_MESSAGE}");

    match subcommand {
        SendMetricSubcommand::Increment(args) => increment::execute(args),
        SendMetricSubcommand::Distribution(args) => distribution::execute(args),
        SendMetricSubcommand::Gauge(args) => gauge::execute(args),
        SendMetricSubcommand::Set(args) => set::execute(args),
    }
}
