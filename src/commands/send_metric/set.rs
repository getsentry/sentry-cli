use super::common_args::CommonMetricArgs;
use crate::{api::envelopes_api::EnvelopesApi, utils::metrics::DefaultTags};
use anyhow::Result;
use clap::{command, Args};
use sentry::metrics::Metric;

#[derive(Args)]
pub(super) struct SetMetricArgs {
    #[command(flatten)]
    common: CommonMetricArgs,

    #[arg(short, long)]
    #[arg(
        help = "Value to add to the set. If the set already contains the provided value, the \
        set's unique count will not increase."
    )]
    value: String,
}

pub(super) fn execute(args: SetMetricArgs) -> Result<()> {
    EnvelopesApi::try_new()?.send_envelope(
        Metric::set(args.common.name, &args.value)
            .with_unit(args.common.unit)
            .with_tags(args.common.tags.with_default_tags())
            .finish()
            .to_envelope(),
    )?;
    Ok(())
}
