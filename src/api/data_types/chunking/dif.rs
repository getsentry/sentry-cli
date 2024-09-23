use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha1_smol::Digest;
use symbolic::common::DebugId;

use crate::api::DebugInfoFile;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedDifRequest<'a> {
    pub name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_id: Option<DebugId>,
    pub chunks: &'a [Digest],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkedDifResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
    pub dif: Option<DebugInfoFile>,
}

pub type AssembleDifsRequest<'a> = HashMap<Digest, ChunkedDifRequest<'a>>;
pub type AssembleDifsResponse = HashMap<Digest, ChunkedDifResponse>;
