use std::num::NonZeroUsize;

use serde::Deserialize;

use super::{ChunkCompression, ChunkHashAlgorithm};

/// Chunk upload options which are set by the Sentry server.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkServerOptions {
    pub url: String,
    #[serde(rename = "chunksPerRequest")]
    pub max_chunks: u64,
    #[serde(rename = "maxRequestSize")]
    pub max_size: u64,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default)]
    pub max_wait: u64,
    #[expect(dead_code)]
    pub hash_algorithm: ChunkHashAlgorithm,
    pub chunk_size: NonZeroUsize,
    pub concurrency: u8,
    #[serde(default)]
    pub compression: Vec<ChunkCompression>,
}
