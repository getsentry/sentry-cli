//! This file contains code for enabling chunk uploads for Proguard mappings.
//!
//! This code is intended as a temporary solution to enable chunk uploads for
//! Proguard mappings, while we work on a more permanent solution, which will
//! work for all different types of debug files.

use std::time::Duration;

use anyhow::Result;

use crate::api::ChunkServerOptions;
use crate::utils::chunks::{upload_chunked_objects, ChunkOptions, Chunked};
use crate::utils::proguard::ProguardMapping;

/// How long to wait for the server to assemble the mappings before giving up.
// 120 seconds was chosen somewhat arbitrarily, but in my testing, assembly
// usually was almost instantaneous, so this should probably be enough time.
const ASSEMBLE_POLL_TIMEOUT: Duration = Duration::from_secs(120);

/// Uploads a set of Proguard mappings to Sentry.
/// Blocks until the mappings have been assembled (up to ASSEMBLE_POLL_TIMEOUT).
/// Returns an error if the mappings fail to assemble, or if the timeout is reached.
pub fn chunk_upload(
    mappings: &[ProguardMapping<'_>],
    chunk_upload_options: ChunkServerOptions,
    org: &str,
    project: &str,
) -> Result<()> {
    let chunked_mappings = mappings
        .iter()
        .map(|mapping| Chunked::from(mapping, chunk_upload_options.chunk_size as usize))
        .collect::<Result<Vec<_>>>()?;

    let options =
        ChunkOptions::new(chunk_upload_options, org, project).with_max_wait(ASSEMBLE_POLL_TIMEOUT);

    let (_, has_processing_errors) = upload_chunked_objects(&chunked_mappings, options)?;

    if has_processing_errors {
        Err(anyhow::anyhow!("Some symbols did not process correctly"))
    } else {
        Ok(())
    }
}
