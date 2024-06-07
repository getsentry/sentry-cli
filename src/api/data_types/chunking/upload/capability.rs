use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkUploadCapability {
    /// Chunked upload of debug files
    DebugFiles,

    /// Chunked upload of release files
    ReleaseFiles,

    /// Chunked upload of standalone artifact bundles
    ArtifactBundles,

    /// Like `ArtifactBundles`, but with deduplicated chunk
    /// upload.
    ArtifactBundlesV2,

    /// Upload of PDBs and debug id overrides
    Pdbs,

    /// Upload of Portable PDBs
    PortablePdbs,

    /// Uploads of source archives
    Sources,

    /// Upload of BCSymbolMap and PList auxiliary DIFs
    BcSymbolmap,

    /// Upload of il2cpp line mappings
    Il2Cpp,

    /// Any other unsupported capability (ignored)
    Unknown,
}

impl<'de> Deserialize<'de> for ChunkUploadCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "debug_files" => ChunkUploadCapability::DebugFiles,
            "release_files" => ChunkUploadCapability::ReleaseFiles,
            "artifact_bundles" => ChunkUploadCapability::ArtifactBundles,
            "artifact_bundles_v2" => ChunkUploadCapability::ArtifactBundlesV2,
            "pdbs" => ChunkUploadCapability::Pdbs,
            "portablepdbs" => ChunkUploadCapability::PortablePdbs,
            "sources" => ChunkUploadCapability::Sources,
            "bcsymbolmaps" => ChunkUploadCapability::BcSymbolmap,
            "il2cpp" => ChunkUploadCapability::Il2Cpp,
            _ => ChunkUploadCapability::Unknown,
        })
    }
}
