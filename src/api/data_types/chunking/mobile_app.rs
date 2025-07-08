#![cfg(feature = "unstable-mobile-app")]
use serde::{Deserialize, Serialize};
use sha1_smol::Digest;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedMobileAppRequest<'a> {
    pub checksum: Digest,
    pub chunks: &'a [Digest],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_configuration: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssembleMobileAppResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
}
