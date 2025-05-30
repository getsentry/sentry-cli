use serde::{Deserialize, Serialize};
use sha1_smol::Digest;
use std::collections::HashMap;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkedArtifactResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct AssembleArtifactsRequest<'a>(HashMap<Digest, ChunkedArtifactRequest<'a>>);

impl<'a, T> FromIterator<T> for AssembleArtifactsRequest<'a>
where
    T: Into<ChunkedArtifactRequest<'a>>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self(
            iter.into_iter()
                .map(|obj| obj.into())
                .map(|r| (r.checksum, r))
                .collect(),
        )
    }
}

pub type AssembleArtifactsResponse = ChunkedArtifactResponse;

#[derive(Debug, Serialize)]
pub struct ChunkedPreprodArtifactRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    // Optional metadata fields that the server supports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_configuration: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_built: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<serde_json::Value>,
}

impl<'a> ChunkedPreprodArtifactRequest<'a> {
    /// Create a new ChunkedPreprodArtifactRequest with the required fields.
    pub fn new(checksum: Digest, chunks: &'a [Digest]) -> Self {
        Self {
            checksum,
            chunks,
            build_version: None,
            build_number: None,
            build_configuration: None,
            date_built: None,
            extras: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkedPreprodArtifactResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
}

pub type AssemblePreprodArtifactsResponse = HashMap<Digest, ChunkedPreprodArtifactResponse>;

fn version_is_empty(version: &Option<&str>) -> bool {
    match version {
        Some(v) => v.is_empty(),
        None => true,
    }
}
