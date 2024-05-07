use super::common_args::CommonMetricArgs;
use crate::utils::metrics::{
    normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
};
use anyhow::Result;
use clap::{command, Args};
use log::debug;
use sentry::protocol::EnvelopeItem;

#[derive(Args)]
pub(super) struct IncrementArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(short, long, help = "Metric value", default_value = "1")]
    value: f64,
}

pub(super) fn execute(args: IncrementArgs) -> Result<()> {
    let value = MetricValue::Float(args.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Counter);
    debug!("Sending payload: {}", (payload.to_string()?));
    super::send_envelope(EnvelopeItem::Statsd(payload.to_bytes()?))
}
