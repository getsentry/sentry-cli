use std::fmt;

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Default)]
pub enum ChunkCompression {
    /// No compression should be applied
    #[default]
    Uncompressed = 0,
    /// GZIP compression (including header)
    Gzip = 10,
    /// Brotli compression
    Brotli = 20,
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

impl<'de> Deserialize<'de> for ChunkCompression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "gzip" => ChunkCompression::Gzip,
            "brotli" => ChunkCompression::Brotli,
            // We do not know this compression, so we assume no compression
            _ => ChunkCompression::Uncompressed,
        })
    }
}
