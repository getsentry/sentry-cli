//! Custom collections used in our tests.
//!
//! Ideally, we would eventually move these to their own crate.
//! The only reason we have this in the `tests` directory for now,
//! is that it is only used in tests, so should not be compiled into
//! production code.

use std::collections::HashMap;
use std::hash::Hash;

/// An unordered collection of items that allows duplicates.
/// This is implemented as a `HashMap` where the keys are the items
/// and the values are the counts of the items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMultiSet<T>
where
    T: Hash + Eq,
{
    pub inner: HashMap<T, usize>,
}

impl<T> HashMultiSet<T>
where
    T: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert(&mut self, item: T) {
        *self.inner.entry(item).or_default() += 1;
    }
}

impl<T> FromIterator<T> for HashMultiSet<T>
where
    T: Hash + Eq,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut set = Self::new();
        for item in iter {
            set.insert(item);
        }
        set
    }
}
