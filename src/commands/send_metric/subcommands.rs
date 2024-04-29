use clap::Args;
use clap::{command, Subcommand};

use crate::utils::value_parsers;

#[derive(Subcommand)]
#[command(about = "Send a metric to Sentry.")]
#[command(long_about = "Send a metric event to Sentry.{n}{n}\
This command will validate input parameters and attempt to send a metric to \
Sentry. Due to network errors and rate limits, the metric is not guaranteed to \
arrive.")]
pub enum SendMetricSubcommand {
    #[command(about = "Send an increment to a counter metric")]
    Increment(IncrementArgs),
    #[command(about = "Send a value to a distribution metric")]
    Distribution(FloatValueMetricArgs),
    #[command(about = "Send a value to a gauge metric")]
    Gauge(FloatValueMetricArgs),
    #[command(about = "Send a value to a set metric")]
    Set(SetArgs),
}

#[derive(Args)]
pub struct FloatValueMetricArgs {
    #[command(flatten)]
    pub common: CommonMetricArgs,

    #[arg(short, long, help = "Metric value")]
    pub value: f64,
}

#[derive(Args)]
pub struct IncrementArgs {
    #[command(flatten)]
    pub common: CommonMetricArgs,

    #[arg(short, long, help = "Metric value", default_value = "1")]
    pub value: f64,
}

#[derive(Args)]
pub struct SetArgs {
    #[command(flatten)]
    pub common: CommonMetricArgs,

    #[arg(short, long, value_parser=value_parsers::set_value_parser)]
    #[arg(help = "Metric value: strings and integers are supported, floats are floored")]
    pub value: f64,
}

#[derive(Args)]
pub struct CommonMetricArgs {
    #[arg(short, long, help = "Metric key/name")]
    #[arg(visible_alias = "name", visible_short_alias = 'n')]
    pub key: String,

    #[arg(short, long, help = "Metric unit")]
    pub unit: Option<String>,

    #[arg(short, long, value_delimiter=',', value_name = "KEY:VALUE", num_args = 1..)]
    #[arg(value_parser=value_parsers::kv_parser, help = "Metric tags as key:value pairs")]
    pub tags: Vec<(String, String)>,
}
