use crate::commands::send_metric::subcommands::{
    CommonMetricArgs, FloatValueMetricArgs, IncrementArgs, SendMetricSubcommand, SetArgs,
};

pub(super) struct MetricData {
    pub(super) key: String,
    pub(super) value: f64,
    pub(super) metric_type: char,
    pub(super) unit: Option<String>,
    pub(super) tags: Vec<(String, String)>,
}

impl From<SendMetricSubcommand> for MetricData {
    fn from(subcommand: SendMetricSubcommand) -> Self {
        let (CommonMetricArgs { key, unit, tags }, value, metric_type) = match subcommand {
            SendMetricSubcommand::Increment(IncrementArgs { common, value }) => {
                (common, value, 'c')
            }
            SendMetricSubcommand::Gauge(FloatValueMetricArgs { common, value }) => {
                (common, value, 'g')
            }
            SendMetricSubcommand::Distribution(FloatValueMetricArgs { common, value }) => {
                (common, value, 'd')
            }
            SendMetricSubcommand::Set(SetArgs { common, value }) => (common, value, 's'),
        };
        Self {
            key,
            unit,
            tags,
            metric_type,
            value,
        }
    }
}
