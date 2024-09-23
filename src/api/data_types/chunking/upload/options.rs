#![allow(dead_code)] // `hash_algorithm` is never used

use serde::Deserialize;

use super::{ChunkCompression, ChunkHashAlgorithm, ChunkUploadCapability};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkUploadOptions {
    pub url: String,
    #[serde(rename = "chunksPerRequest")]
    pub max_chunks: u64,
    #[serde(rename = "maxRequestSize")]
    pub max_size: u64,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default)]
    pub max_wait: u64,
    pub hash_algorithm: ChunkHashAlgorithm,
    pub chunk_size: u64,
    pub concurrency: u8,
    #[serde(default)]
    pub compression: Vec<ChunkCompression>,
    #[serde(default = "default_chunk_upload_accept")]
    pub accept: Vec<ChunkUploadCapability>,
}

impl ChunkUploadOptions {
    /// Returns whether the given capability is accepted by the chunk upload endpoint.
    pub fn supports(&self, capability: ChunkUploadCapability) -> bool {
        self.accept.contains(&capability)
    }
}

fn default_chunk_upload_accept() -> Vec<ChunkUploadCapability> {
    vec![ChunkUploadCapability::DebugFiles]
}
