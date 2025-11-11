use serde::{Deserialize, Serialize};
use sha1_smol::Digest;

use crate::api::VcsInfo;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedBuildRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_configuration: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<&'a str>,
    #[serde(flatten)]
    pub vcs_info: &'a VcsInfo<'a>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleBuildResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
    pub artifact_url: Option<String>,
}
