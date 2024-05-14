use super::common_args::CommonMetricArgs;
use crate::{
    api::envelopes_api::EnvelopesApi,
    utils::metrics::{
        arg_parsers, normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
    },
};
use anyhow::Result;
use clap::{command, Args};
use sentry::protocol::EnvelopeItem;

#[derive(Args)]
pub(super) struct SetArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(short, long, value_parser=arg_parsers::set_value_parser)]
    #[arg(help = "Metric value")]
    value: i64,
}

pub(super) fn execute(args: SetArgs) -> Result<()> {
    let value = MetricValue::Int(args.value);
    let payload = NormalizedPayload::from_cli_args(&args.common, value, MetricType::Set);
    EnvelopesApi::try_new()?.send_item(EnvelopeItem::Statsd(payload.to_bytes()?))
}
