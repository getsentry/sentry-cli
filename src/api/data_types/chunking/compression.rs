use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkCompression {
    /// GZIP compression (including header)
    Gzip = 10,
    /// Brotli compression
    Brotli = 20,
    /// No compression should be applied
    #[default]
    #[serde(other)]
    Uncompressed = 0,
}

impl ChunkCompression {
    pub(in crate::api) fn field_name(self) -> &'static str {
        match self {
            ChunkCompression::Uncompressed => "file",
            ChunkCompression::Gzip => "file_gzip",
            ChunkCompression::Brotli => "file_brotli",
        }
    }
}

impl fmt::Display for ChunkCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ChunkCompression::Uncompressed => write!(f, "uncompressed"),
            ChunkCompression::Gzip => write!(f, "gzip"),
            ChunkCompression::Brotli => write!(f, "brotli"),
        }
    }
}
