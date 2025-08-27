//! Data types used in the API for sending and receiving data
//! from the server.

mod artifact;
mod build;
mod compression;
mod dif;
mod file_state;
mod hash_algorithm;
mod upload;

pub use self::artifact::{AssembleArtifactsResponse, ChunkedArtifactRequest};
pub use self::build::{AssembleBuildResponse, ChunkedBuildRequest};
pub use self::compression::ChunkCompression;
pub use self::dif::{AssembleDifsRequest, AssembleDifsResponse, ChunkedDifRequest};
pub use self::file_state::ChunkedFileState;
pub use self::hash_algorithm::ChunkHashAlgorithm;
pub use self::upload::{ChunkServerOptions, ChunkUploadCapability};
