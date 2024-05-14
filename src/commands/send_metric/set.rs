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
pub(super) struct SetMetricArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(
        short,
        long,
        help = "Value to add to the set. If the set already contains the provided value, the \
        set's unique count will not increase."
    )]
    value: SetMetricValue,
}

#[derive(Clone)]
struct SetMetricValue {
    value: u32,
}

impl From<String> for SetMetricValue {
    fn from(s: String) -> SetMetricValue {
        SetMetricValue {
            value: crc32fast::hash(s.as_bytes()),
        }
    }
}

pub(super) fn execute(args: SetMetricArgs) -> Result<()> {
    let value = MetricValue::UInt(args.value.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Set);
    EnvelopesApi::try_new()?.send_item(EnvelopeItem::Statsd(payload.to_bytes()?))?;
    Ok(())
}
