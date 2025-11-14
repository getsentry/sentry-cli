use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkUploadCapability {
    /// Upload of Dart symbol maps
    DartSymbolMap,

    /// Upload of preprod artifacts
    PreprodArtifacts,

    /// Upload of ProGuard mappings
    Proguard,

    /// Any other unsupported capability (ignored)
    Unknown,
}

impl<'de> Deserialize<'de> for ChunkUploadCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "dartsymbolmap" => ChunkUploadCapability::DartSymbolMap,
            "preprod_artifacts" => ChunkUploadCapability::PreprodArtifacts,
            "proguard" => ChunkUploadCapability::Proguard,
            _ => ChunkUploadCapability::Unknown,
        })
    }
}
