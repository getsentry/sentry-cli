use super::common_args::FloatValueMetricArgs;
use crate::{api::envelopes_api::EnvelopesApi, utils::metrics::DefaultTags as _};
use anyhow::Result;
use sentry::metrics::Metric;

pub(super) fn execute(args: FloatValueMetricArgs) -> Result<()> {
    EnvelopesApi::try_new()?.send_envelope(
        Metric::distribution(args.common.name, args.value)
            .with_unit(args.common.unit)
            .with_tags(args.common.tags.with_default_tags())
            .finish()
            .to_envelope(),
    )?;
    Ok(())
}
