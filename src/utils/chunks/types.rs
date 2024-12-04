//! Contains data types used in the chunk upload process.

use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};

use anyhow::Result;
use sha1_smol::Digest;
use symbolic::common::DebugId;

use crate::api::ChunkedDifRequest;
use crate::utils::chunks::Chunk;
use crate::utils::fs;

/// Information returned by `assemble_difs` containing flat lists of incomplete
/// objects and their missing chunks.
pub type MissingObjectsInfo<'m, T> = (Vec<&'m Chunked<T>>, Vec<Chunk<'m>>);

/// A trait for objects that can be assembled via the `assemble_difs` endpoint.
pub trait Assemblable {
    /// Returns the name of the object.
    fn name(&self) -> Cow<'_, str>;

    /// Returns the debug ID of the object.
    /// Types which do not have a debug ID should return `None`.
    fn debug_id(&self) -> Option<DebugId>;
}

/// Chunked arbitrary data with computed SHA1 checksums.
pub struct Chunked<T> {
    /// Original object
    object: T,

    /// SHA1 checksum of the entire object
    checksum: Digest,

    /// SHA1 checksums of all chunks
    chunks: Vec<Digest>,

    /// Size of a single chunk
    chunk_size: usize,
}

impl<T> Chunked<T> {
    /// Returns the SHA1 checksum of the entire object.
    pub fn checksum(&self) -> Digest {
        self.checksum
    }

    /// Returns the original object.
    pub fn object(&self) -> &T {
        &self.object
    }

    /// Returns the SHA1 checksums of each chunk, in order.
    pub fn chunk_hashes(&self) -> &[Digest] {
        &self.chunks
    }
}

impl<T> Chunked<T>
where
    T: AsRef<[u8]>,
{
    /// Creates a new `ChunkedObject` from the given object, using
    /// the given chunk size.
    pub fn from(object: T, chunk_size: usize) -> Result<Self> {
        let (checksum, chunks) = fs::get_sha1_checksums(object.as_ref(), chunk_size)?;
        Ok(Self {
            object,
            checksum,
            chunks,
            chunk_size,
        })
    }

    /// Returns an iterator over all chunks of the object.
    /// The iterator yields `Chunk` objects, which contain the chunk's
    /// SHA1 checksum and a byte slice pointing to the chunk's data.
    pub fn iter_chunks(&self) -> impl Iterator<Item = Chunk<'_>> {
        self.object
            .as_ref()
            .chunks(self.chunk_size)
            .zip(self.chunk_hashes().iter())
            .map(|(data, checksum)| Chunk((*checksum, data)))
    }
}

impl<T> Display for Chunked<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.object())
    }
}

impl<T> Assemblable for &T
where
    T: Assemblable,
{
    fn name(&self) -> Cow<str> {
        (*self).name()
    }

    fn debug_id(&self) -> Option<DebugId> {
        (*self).debug_id()
    }
}

impl<T> Assemblable for Chunked<T>
where
    T: Assemblable,
{
    fn name(&self) -> Cow<'_, str> {
        self.object().name()
    }

    fn debug_id(&self) -> Option<DebugId> {
        self.object().debug_id()
    }
}

impl<'c, T> From<&'c Chunked<T>> for ChunkedDifRequest<'c>
where
    T: Assemblable,
{
    fn from(value: &'c Chunked<T>) -> Self {
        Self::new(value.name(), value.chunk_hashes(), value.checksum())
            .with_debug_id(value.debug_id())
    }
}
