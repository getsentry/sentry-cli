use crate::utils::metrics::arg_parsers;
use crate::utils::value_parsers;
use clap::command;
use clap::Args;

#[derive(Args)]
pub(super) struct FloatValueMetricArgs {
    #[command(flatten)]
    pub(super) common: CommonMetricArgs,

    #[arg(short, long, help = "Metric value")]
    pub(super) value: f64,
}

#[derive(Args)]
pub struct CommonMetricArgs {
    #[arg(short, long, visible_alias = "name", visible_short_alias = 'n')]
    #[arg(help = "Metric key/name", value_parser = arg_parsers::key_parser)]
    pub key: String,

    #[arg(short, long, help = "Metric unit")]
    pub unit: Option<String>,

    #[arg(short, long, value_delimiter=',', value_name = "KEY:VALUE", num_args = 1..)]
    #[arg(value_parser=value_parsers::kv_parser, help = "Metric tags as key:value pairs")]
    pub tags: Vec<(String, String)>,
}
