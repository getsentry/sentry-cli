use super::common_args::FloatValueMetricArgs;
use crate::utils::metrics::{
    normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
};
use anyhow::Result;
use log::debug;
use sentry::protocol::EnvelopeItem;

pub(super) fn execute(args: FloatValueMetricArgs) -> Result<()> {
    let value = MetricValue::Float(args.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Gauge);
    debug!("Sending payload: {}", (payload.to_string()?));
    super::send_envelope(EnvelopeItem::Statsd(payload.to_bytes()?))
}
