use super::common_args::CommonMetricArgs;
use crate::{
    api::envelopes_api::EnvelopesApi,
    utils::metrics::{
        normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
    },
};
use anyhow::Result;
use clap::{command, Args};
use sentry::protocol::EnvelopeItem;

#[derive(Args)]
pub(super) struct IncrementMetricArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(
        short,
        long,
        help = "Metric value, any finite 64 bit float.",
        default_value = "1"
    )]
    value: f64,
}

pub(super) fn execute(args: IncrementMetricArgs) -> Result<()> {
    let value = MetricValue::Float(args.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Counter);
    EnvelopesApi::try_new()?.send_item(EnvelopeItem::Statsd(payload.to_bytes()?))?;
    Ok(())
}
