//! Contains the `BatchedSliceExt` trait which allows iterating slices in
//! batches of a maximum combined item size or maximum item count, each.
//!
//! In order to support this trait, items must implement the `ItemSize` trait.
//!
//! See `BatchedSliceExt::batches` for more information.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::info;
use parking_lot::RwLock;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use sha1_smol::Digest;

use crate::api::{Api, ChunkUploadOptions, ProgressBarMode};
use crate::utils::progress::{ProgressBar, ProgressStyle};

/// Timeout for polling all assemble endpoints.
pub const ASSEMBLE_POLL_INTERVAL: Duration = Duration::from_millis(1000);

/// A trait required by `BatchedSliceExt` to determine the logical size of a
/// batch. Semantics and unit of the size depend on the batching use case,
/// likely a number of bytes. See `BatchedSliceExt` for more information.
///
/// `ItemSize` is automatically implemented for all numbers convertible to u64.
pub trait ItemSize {
    /// Returns the logical size of this item.
    fn size(&self) -> u64;
}

impl<T> ItemSize for T
where
    T: Into<u64> + Copy,
{
    fn size(&self) -> u64 {
        (*self).into()
    }
}

/// An iterator over a slice in continuous, non-overlapping batches. Each batch
/// contains items with a combined size of up to `max_size`, but at least one
/// item (possibly exceeding `max_size`) and at most `max_items`.
///
/// When the item size is not evenly distributed, the last batch will contain
/// the remainder.
///
/// This struct is created by `BatchedSliceExt::batches`.
pub struct Batches<'data, T>
where
    T: ItemSize,
{
    items: &'data [T],
    max_size: u64,
    max_items: u64,
    index: usize,
}

impl<'data, T> Batches<'data, T>
where
    T: ItemSize + 'data,
{
    pub fn new(items: &'data [T], max_size: u64, max_items: u64) -> Batches<'data, T> {
        Batches {
            items,
            max_size,
            max_items,
            index: 0,
        }
    }
}

impl<'data, T> Iterator for Batches<'data, T>
where
    T: ItemSize + 'data,
{
    type Item = (&'data [T], u64);

    fn next(&mut self) -> Option<Self::Item> {
        // `start` is the first index of this batch, `self.index` will point to
        // after the last item
        let start = self.index;
        let mut size = 0;

        if start >= self.items.len() {
            return None;
        }

        // Iterate until the batch exceeds `max_items` or reaches the end
        while self.index - start < self.max_items as usize && self.index < self.items.len() {
            // Only if there is more than one element in this batch, have a look
            // at the next element and return if it would exceed `max_size`.
            let next = self.items[self.index].size();
            if start < self.index && size + next > self.max_size {
                break;
            }

            self.index += 1;
            size += next;
        }

        Some((&self.items[start..self.index], size))
    }
}

/// An extension to slices that allows iteration over sized batches.
pub trait BatchedSliceExt<T: ItemSize> {
    /// Returns an iterator over continuous batches of items with a combined
    /// size of up to `max_size` but at least one item (possibly exceeding
    /// `max_size`) and at most `max_items`. Items must implement `ItemSize`.
    ///
    /// ```
    /// use utils::batch::{BatchedSliceExt, ItemSize};
    ///
    /// let slice = &[5, 10, 1, 1, 1, 1];
    /// let mut batches = slice.batches(5, 3);
    /// assert_eq!(batches.next(), &[5]);
    /// assert_eq!(batches.next(), &[10]);
    /// assert_eq!(batches.next(), &[1, 1, 1]);
    /// assert_eq!(batches.next(), &[1]);
    /// ```
    fn batches(&self, max_size: u64, max_items: u64) -> Batches<'_, T>;
}

impl<S, T> BatchedSliceExt<T> for S
where
    S: AsRef<[T]>,
    T: ItemSize,
{
    fn batches(&self, max_size: u64, max_items: u64) -> Batches<'_, T> {
        Batches::new(self.as_ref(), max_size, max_items)
    }
}

/// A single chunk of a file to upload. It carries the binary data slice and a SHA1 checksum of that
/// data.
///
/// `Chunk` implements `AsRef<(Digest, &[u8])>` so that it can be easily transformed into a vector
/// or map.
#[derive(Debug)]
pub struct Chunk<'data>(pub (Digest, &'data [u8]));

impl<'data> AsRef<(Digest, &'data [u8])> for Chunk<'data> {
    fn as_ref(&self) -> &(Digest, &'data [u8]) {
        &self.0
    }
}

impl<'data> ItemSize for Chunk<'data> {
    fn size(&self) -> u64 {
        (self.0).1.len() as u64
    }
}

/// Concurrently uploads chunks in batches. The batch size and number of concurrent requests is
/// controlled by `chunk_options`.
///
/// This function blocks until all chunks have been uploaded.
pub fn upload_chunks(
    chunks: &[Chunk<'_>],
    chunk_options: &ChunkUploadOptions,
    progress_style: ProgressStyle,
) -> Result<()> {
    let total_bytes = chunks.iter().map(|&Chunk((_, data))| data.len()).sum();

    // Chunks are uploaded in batches, but the progress bar is shared between
    // multiple requests to simulate one continuous upload to the user. Since we
    // have to embed the progress bar into a ProgressBarMode and move it into
    // `Api::upload_chunks`, the progress bar is created in an Arc.
    let pb = Arc::new(ProgressBar::new(total_bytes));
    pb.set_style(progress_style);

    // Select the best available compression mechanism. We assume that every
    // compression algorithm has been implemented for uploading, except `Other`
    // which is used for unknown compression algorithms. In case the server
    // does not support compression, we fall back to `Uncompressed`.
    let compression = chunk_options
        .compression
        .iter()
        .max()
        .cloned()
        .unwrap_or_default();

    info!("using '{}' compression for chunk upload", compression);

    // The upload is executed in parallel batches. Each batch aggregates objects
    // until it exceeds the maximum size configured in ChunkUploadOptions. We
    // keep track of the overall progress and potential errors. If an error
    // ocurrs, all subsequent requests will be cancelled and the error returned.
    // Otherwise, the after every successful update, the overall progress is
    // updated and rendered.
    let batches: Vec<_> = chunks
        .batches(chunk_options.max_size, chunk_options.max_chunks)
        .collect();

    // We count the progress of each batch separately to avoid synchronization
    // issues. For a more consistent progress bar in repeated uploads, we also
    // add the already uploaded bytes to the progress bar.
    let bytes = Arc::new(RwLock::new(vec![0u64; batches.len()]));

    let pool = ThreadPoolBuilder::new()
        .num_threads(chunk_options.concurrency as usize)
        .build()?;

    pool.install(|| {
        batches
            .into_par_iter()
            .enumerate()
            .map(|(index, (batch, size))| {
                let mode = ProgressBarMode::Shared((pb.clone(), size, index, bytes.clone()));
                Api::current().upload_chunks(&chunk_options.url, batch, mode, compression)
            })
            .collect::<Result<(), _>>()
    })?;

    pb.finish_with_duration("Uploading");

    Ok(())
}
