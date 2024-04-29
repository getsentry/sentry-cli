use crate::commands::send_metric::subcommands::{
    CommonMetricArgs, FloatValueMetricArgs, IncrementArgs, SendMetricSubcommand, SetArgs,
};

impl SendMetricSubcommand {
    pub(super) fn metric_type(&self) -> char {
        match self {
            SendMetricSubcommand::Increment(_) => 'c',
            SendMetricSubcommand::Distribution(_) => 'd',
            SendMetricSubcommand::Gauge(_) => 'g',
            SendMetricSubcommand::Set(_) => 's',
        }
    }

    pub(super) fn value(&self) -> f64 {
        match self {
            SendMetricSubcommand::Increment(IncrementArgs { value, .. })
            | SendMetricSubcommand::Gauge(FloatValueMetricArgs { value, .. })
            | SendMetricSubcommand::Distribution(FloatValueMetricArgs { value, .. })
            | SendMetricSubcommand::Set(SetArgs { value, .. }) => *value,
        }
    }
}

impl From<SendMetricSubcommand> for CommonMetricArgs {
    fn from(subcommand: SendMetricSubcommand) -> Self {
        match subcommand {
            SendMetricSubcommand::Increment(IncrementArgs { common, .. })
            | SendMetricSubcommand::Gauge(FloatValueMetricArgs { common, .. })
            | SendMetricSubcommand::Distribution(FloatValueMetricArgs { common, .. })
            | SendMetricSubcommand::Set(SetArgs { common, .. }) => common,
        }
    }
}
