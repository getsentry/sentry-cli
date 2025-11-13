use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha1_smol::Digest;
use symbolic::common::DebugId;

use crate::api::DebugInfoFile;

use super::ChunkedFileState;

#[derive(Debug, Serialize)]
pub struct ChunkedDifRequest<'a> {
    pub name: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_id: Option<DebugId>,
    pub chunks: &'a [Digest],
    #[serde(skip)]
    hash: Digest,
}

impl<'a> ChunkedDifRequest<'a> {
    /// Create a new ChunkedDifRequest with the given name, chunk hashes,
    /// and total hash for the entire file.
    pub fn new(name: Cow<'a, str>, chunks: &'a [Digest], hash: Digest) -> Self {
        Self {
            name,
            chunks,
            hash,
            debug_id: None,
        }
    }

    /// Set the provided debug_id on the request, or clear it if
    /// `None` is passed.
    pub fn with_debug_id(mut self, debug_id: Option<DebugId>) -> Self {
        self.debug_id = debug_id;
        self
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkedDifResponse {
    pub state: ChunkedFileState,
    pub missing_chunks: Vec<Digest>,
    pub detail: Option<String>,
    pub dif: Option<DebugInfoFile>,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct AssembleDifsRequest<'a>(HashMap<Digest, ChunkedDifRequest<'a>>);

impl<'a, T> FromIterator<T> for AssembleDifsRequest<'a>
where
    T: Into<ChunkedDifRequest<'a>>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self(
            iter.into_iter()
                .map(|obj| obj.into())
                .map(|r| (r.hash, r))
                .collect(),
        )
    }
}

pub type AssembleDifsResponse = HashMap<Digest, ChunkedDifResponse>;
