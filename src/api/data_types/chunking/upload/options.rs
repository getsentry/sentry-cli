use serde::Deserialize;

use super::{ChunkCompression, ChunkHashAlgorithm, ChunkUploadCapability};

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
    pub chunk_size: u64,
    pub concurrency: u8,
    #[serde(default)]
    pub compression: Vec<ChunkCompression>,
    #[serde(default = "default_chunk_upload_accept")]
    pub accept: Vec<ChunkUploadCapability>,
}

impl ChunkServerOptions {
    /// Returns whether the given capability is accepted by the chunk upload endpoint.
    pub fn supports(&self, capability: ChunkUploadCapability) -> bool {
        self.accept.contains(&capability)
    }

    /// Determines whether we need to strip debug_ids from the requests. We need
    /// to strip the debug_ids whenever the server does not support chunked
    /// uploading of PDBs, to maintain backwards compatibility.
    ///
    /// See: https://github.com/getsentry/sentry-cli/issues/980
    /// See: https://github.com/getsentry/sentry-cli/issues/1056
    pub fn should_strip_debug_ids(&self) -> bool {
        !self.supports(ChunkUploadCapability::DebugFiles)
    }
}

fn default_chunk_upload_accept() -> Vec<ChunkUploadCapability> {
    vec![ChunkUploadCapability::DebugFiles]
}
