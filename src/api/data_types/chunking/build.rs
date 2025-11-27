use serde::{Deserialize, Serialize};
use sha1_smol::Digest;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedBuildRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_configuration: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_built: Option<&'a str>,
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

/// VCS information for build app uploads
#[derive(Debug, Serialize)]
pub struct VcsInfo<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_sha: Option<Digest>,
    #[serde(skip_serializing_if = "str::is_empty", rename = "provider")]
    pub vcs_provider: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub head_repo_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub base_repo_name: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub head_ref: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub base_ref: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<&'a u32>,
}
