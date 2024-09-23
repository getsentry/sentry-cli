use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ChunkHashAlgorithm {
    #[serde(rename = "sha1")]
    Sha1,
}
