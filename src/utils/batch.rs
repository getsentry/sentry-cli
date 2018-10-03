//! Contains the `BatchedSliceExt` trait which allows iterating slices in
//! batches of a maximum combined item size or maximum item count, each.
//!
//! In order to support this trait, items must implement the `ItemSize` trait.
//!
//! See `BatchedSliceExt::batches` for more information.

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
        (*self).into() as u64
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
    T: ItemSize + 'data,
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
    fn batches(&self, max_size: u64, max_items: u64) -> Batches<T>;
}

impl<S, T> BatchedSliceExt<T> for S
where
    S: AsRef<[T]>,
    T: ItemSize,
{
    fn batches(&self, max_size: u64, max_items: u64) -> Batches<T> {
        Batches::new(self.as_ref(), max_size, max_items)
    }
}
