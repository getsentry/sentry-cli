//! This file contains code for enabling chunk uploads for Proguard mappings.
//!
//! This code is intended as a temporary solution to enable chunk uploads for
//! Proguard mappings, while we work on a more permanent solution, which will
//! work for all different types of debug files.

use std::{fs, thread, time::Duration};

use anyhow::{bail, Result};
use indicatif::ProgressStyle;
use sha1_smol::Digest;

use super::chunks;
use super::chunks::Chunk;
use super::fs::get_sha1_checksums;
use crate::api::{Api, ChunkUploadOptions, ChunkedDifRequest, ChunkedFileState};
use crate::commands::upload_proguard::MappingRef;

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
        println!("There are {} chunks", chunk_hashes.len());
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

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "Uploading Proguard mappings...\
             \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})"
    ));

    let chunks = chunked_mappings.iter().flat_map(|mapping| mapping.chunks());

    chunks::upload_chunks(
        &chunks.collect::<Vec<_>>(),
        chunk_upload_options,
        progress_style,
    )?;

    let request = chunked_mappings.iter().map(to_assemble).collect();
    loop {
        let okay = Api::current()
            .authenticated()?
            .assemble_difs(org, project, &request)?
            .iter()
            .map(|(_, response)| {
                Ok(match response.state {
                    ChunkedFileState::Error => bail!(format!("Error: {:?}", response)),
                    ChunkedFileState::NotFound => bail!("File not found"),
                    ChunkedFileState::Created => {
                        println!("Created");
                        false
                    }
                    ChunkedFileState::Assembling => {
                        println!("Assembling");
                        false
                    }
                    ChunkedFileState::Ok => true,
                })
            })
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|&done| done);

        if okay {
            println!("Uploaded");
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
