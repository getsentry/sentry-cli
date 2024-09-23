use serde::{Deserialize, Serialize};
use sha1_smol::Digest;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedArtifactRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dist: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleArtifactsResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
}
