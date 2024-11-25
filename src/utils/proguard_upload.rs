//! This file contains code for enabling chunk uploads for Proguard mappings.
//!
//! This code is intended as a temporary solution to enable chunk uploads for
//! Proguard mappings, while we work on a more permanent solution, which will
//! work for all different types of debug files.

use std::time::{Duration, Instant};
use std::{fs, thread};

use anyhow::Result;
use indicatif::ProgressStyle;
use sha1_smol::Digest;

use super::chunks;
use super::chunks::Chunk;
use super::fs::get_sha1_checksums;
use crate::api::{Api, ChunkUploadOptions, ChunkedDifRequest, ChunkedFileState};
use crate::commands::upload_proguard::MappingRef;

/// How often to poll the server for the status of the assembled mappings.
const ASSEMBLE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How long to wait for the server to assemble the mappings before giving up.
// 120 seconds was chosen somewhat arbitrarily, but in my testing, assembly
// usually was almost instantaneous, so this should probably be enough time.
const ASSEMBLE_POLL_TIMEOUT: Duration = Duration::from_secs(120);

struct ChunkedMapping {
    raw_data: Vec<u8>,
    hash: Digest,
    chunk_hashes: Vec<Digest>,
    file_name: String,
    chunk_size: usize,
}

impl ChunkedMapping {
    fn try_from_mapping(mapping: &MappingRef, chunk_size: u64) -> Result<Self> {
        let raw_data = fs::read(mapping)?;
        let file_name = format!("/proguard/{}.txt", mapping.uuid);

        let (hash, chunk_hashes) = get_sha1_checksums(&raw_data, chunk_size)?;
        Ok(Self {
            raw_data,
            hash,
            chunk_hashes,
            file_name,
            chunk_size: chunk_size.try_into()?,
        })
    }

    fn chunks(&self) -> impl Iterator<Item = Chunk<'_>> {
        self.raw_data
            .chunks(self.chunk_size)
            .zip(self.chunk_hashes.iter())
            .map(|(chunk, hash)| Chunk((*hash, chunk)))
    }
}

impl<'a> From<&'a ChunkedMapping> for ChunkedDifRequest<'a> {
    fn from(value: &'a ChunkedMapping) -> Self {
        ChunkedDifRequest {
            name: &value.file_name,
            debug_id: None,
            chunks: &value.chunk_hashes,
        }
    }
}

fn to_assemble(chunked: &ChunkedMapping) -> (Digest, ChunkedDifRequest<'_>) {
    (chunked.hash, chunked.into())
}

/// Uploads a set of Proguard mappings to Sentry.
/// Blocks until the mappings have been assembled (up to ASSEMBLE_POLL_TIMEOUT).
/// Returns an error if the mappings fail to assemble, or if the timeout is reached.
pub fn chunk_upload(
    paths: &[MappingRef],
    chunk_upload_options: &ChunkUploadOptions,
    org: &str,
    project: &str,
) -> Result<()> {
    let chunked_mappings: Vec<ChunkedMapping> = paths
        .iter()
        .map(|path| ChunkedMapping::try_from_mapping(path, chunk_upload_options.chunk_size))
        .collect::<Result<_>>()?;

    let progress_style = ProgressStyle::default_bar().template(
        "Uploading Proguard mappings...\
             \n{wide_bar}  {bytes}/{total_bytes} ({eta})",
    );

    let chunks = chunked_mappings.iter().flat_map(|mapping| mapping.chunks());

    chunks::upload_chunks(
        &chunks.collect::<Vec<_>>(),
        chunk_upload_options,
        progress_style,
    )?;

    println!("Waiting for server to assemble uploaded mappings...");

    let assemble_request = chunked_mappings.iter().map(to_assemble).collect();
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
