use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkUploadCapability {
    /// Chunked upload of standalone artifact bundles
    ArtifactBundles,

    /// Like `ArtifactBundles`, but with deduplicated chunk
    /// upload.
    ArtifactBundlesV2,

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
            "artifact_bundles" => ChunkUploadCapability::ArtifactBundles,
            "artifact_bundles_v2" => ChunkUploadCapability::ArtifactBundlesV2,
            "dartsymbolmap" => ChunkUploadCapability::DartSymbolMap,
            "preprod_artifacts" => ChunkUploadCapability::PreprodArtifacts,
            "proguard" => ChunkUploadCapability::Proguard,
            _ => ChunkUploadCapability::Unknown,
        })
    }
}
