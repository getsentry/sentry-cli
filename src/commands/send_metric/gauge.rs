use super::common_args::FloatValueMetricArgs;
use crate::{
    api::envelopes_api::EnvelopesApi,
    utils::metrics::{
        normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
    },
};
use anyhow::Result;
use sentry::protocol::EnvelopeItem;

pub(super) fn execute(args: FloatValueMetricArgs) -> Result<()> {
    let value = MetricValue::Float(args.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Gauge);
    EnvelopesApi::try_new()?.send_item(EnvelopeItem::Statsd(payload.to_bytes()?))?;
    Ok(())
}
