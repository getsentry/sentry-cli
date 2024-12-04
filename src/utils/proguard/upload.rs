//! This file contains code for enabling chunk uploads for Proguard mappings.
//!
//! This code is intended as a temporary solution to enable chunk uploads for
//! Proguard mappings, while we work on a more permanent solution, which will
//! work for all different types of debug files.

use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::ProgressStyle;

use crate::api::{Api, ChunkUploadOptions, ChunkedFileState};
use crate::utils::chunks;
use crate::utils::chunks::Chunked;
use crate::utils::proguard::ProguardMapping;

/// How often to poll the server for the status of the assembled mappings.
const ASSEMBLE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How long to wait for the server to assemble the mappings before giving up.
// 120 seconds was chosen somewhat arbitrarily, but in my testing, assembly
// usually was almost instantaneous, so this should probably be enough time.
const ASSEMBLE_POLL_TIMEOUT: Duration = Duration::from_secs(120);

/// Uploads a set of Proguard mappings to Sentry.
/// Blocks until the mappings have been assembled (up to ASSEMBLE_POLL_TIMEOUT).
/// Returns an error if the mappings fail to assemble, or if the timeout is reached.
pub fn chunk_upload(
    mappings: &[ProguardMapping<'_>],
    chunk_upload_options: &ChunkUploadOptions,
    org: &str,
    project: &str,
) -> Result<()> {
    let chunked_mappings = mappings
        .iter()
        .map(|mapping| Chunked::from(mapping, chunk_upload_options.chunk_size as usize))
        .collect::<Result<Vec<_>>>()?;

    let progress_style = ProgressStyle::default_bar().template(
        "Uploading Proguard mappings...\
             \n{wide_bar}  {bytes}/{total_bytes} ({eta})",
    );

    let chunks = chunked_mappings
        .iter()
        .flat_map(|mapping| mapping.iter_chunks());

    chunks::upload_chunks(
        &chunks.collect::<Vec<_>>(),
        chunk_upload_options,
        progress_style,
    )?;

    println!("Waiting for server to assemble uploaded mappings...");

    let assemble_request = chunked_mappings.iter().collect();
    let start = Instant::now();
    while Instant::now().duration_since(start) < ASSEMBLE_POLL_TIMEOUT {
        let all_assembled = Api::current()
            .authenticated()?
            .assemble_difs(org, project, &assemble_request)?
            .values()
            .map(|response| match response.state {
                ChunkedFileState::Error => anyhow::bail!("Error: {response:?}"),
                ChunkedFileState::NotFound => anyhow::bail!("File not found: {response:?}"),
                ChunkedFileState::Ok | ChunkedFileState::Created | ChunkedFileState::Assembling => {
                    Ok(response)
                }
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|response| matches!(response.state, ChunkedFileState::Ok));

        if all_assembled {
            println!("Server finished assembling mappings.");
            return Ok(());
        }

        thread::sleep(ASSEMBLE_POLL_INTERVAL);
    }

    anyhow::bail!("Timed out waiting for server to assemble uploaded mappings.")
}
