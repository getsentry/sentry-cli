use super::common_args::CommonMetricArgs;
use crate::{api::envelopes_api::EnvelopesApi, utils::metrics::DefaultTags};
use anyhow::Result;
use clap::{command, Args};
use sentry::metrics::Metric;

#[derive(Args)]
pub(super) struct IncrementMetricArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(short, long, default_value = "1")]
    #[arg(help = "Value to increment the metric by, any finite 64 bit float.")]
    value: f64,
}

pub(super) fn execute(args: IncrementMetricArgs) -> Result<()> {
    EnvelopesApi::try_new()?.send_envelope(
        Metric::incr(args.common.name, args.value)
            .with_unit(args.common.unit)
            .with_tags(args.common.tags.with_default_tags())
            .finish()
            .to_envelope(),
    )?;
    Ok(())
}
