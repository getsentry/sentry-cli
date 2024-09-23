use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ChunkedFileState {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "assembling")]
    Assembling,
    #[serde(rename = "ok")]
    Ok,
}

impl ChunkedFileState {
    pub fn is_finished(self) -> bool {
        self == ChunkedFileState::Error || self == ChunkedFileState::Ok
    }

    pub fn is_pending(self) -> bool {
        !self.is_finished()
    }

    pub fn is_err(self) -> bool {
        self == ChunkedFileState::Error || self == ChunkedFileState::NotFound
    }
}
