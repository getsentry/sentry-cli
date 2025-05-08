use serde::{Deserialize, Serialize};
use sha1_smol::Digest;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedArtifactRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub projects: &'a [String],
    #[serde(skip_serializing_if = "version_is_empty")]
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

fn version_is_empty(version: &Option<&str>) -> bool {
    match version {
        Some(v) => v.is_empty(),
        None => true,
    }
}
