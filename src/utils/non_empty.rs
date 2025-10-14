//! Contains a type for non-empty vectors.

use std::ops::Deref;

use serde::Serialize;
use thiserror::Error;

/// A slice that is guaranteed to be non-empty.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct NonEmptySlice<'s, T> {
    slice: &'s [T],
}

/// A vector that is guaranteed to be non-empty.
#[derive(Debug)]
pub struct NonEmptyVec<T> {
    vec: Vec<T>,
}

/// Type for errors that occur when creating a non-empty collection from this module.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum NonEmptyError {
    #[error("collection is empty")]
    Empty,
}

impl<'s, T> NonEmptySlice<'s, T> {
    /// Safety: The slice must be non-empty.
    unsafe fn new_unchecked(slice: &'s [T]) -> Self {
        NonEmptySlice { slice }
    }
}

impl<T> NonEmptyVec<T> {
    /// Safety: The vector must be non-empty.
    unsafe fn new_unchecked(vec: Vec<T>) -> Self {
        Self { vec }
    }

    /// Returns a `NonEmptySlice` from this vector.
    pub fn as_non_empty_slice(&self) -> NonEmptySlice<'_, T> {
        NonEmptySlice {
            slice: self.vec.as_slice(),
        }
    }
}

impl<T> Deref for NonEmptyVec<T> {
    type Target = Vec<T>;

    /// Dereferences to the underlying vector.
    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptyVec<T> {
    type Error = NonEmptyError;

    /// Tries to convert a `Vec<T>` into a `NonEmptyVec<T>`, erroring if the vector is empty.
    fn try_from(vec: Vec<T>) -> Result<Self, Self::Error> {
        if vec.is_empty() {
            return Err(NonEmptyError::Empty);
        }

        // Safety: Vector checked for non-emptiness above.
        Ok(unsafe { Self::new_unchecked(vec) })
    }
}

impl<T> From<[T; 1]> for NonEmptyVec<T> {
    /// Converts a single-element array into a `NonEmptyVec<T>`.
    fn from(slice: [T; 1]) -> Self {
        // Safety: Slice has exactly one element.
        unsafe { Self::new_unchecked(slice.into()) }
    }
}

impl<T> From<NonEmptyVec<T>> for Vec<T> {
    /// Converts a `NonEmptyVec<T>` into a `Vec<T>`.
    fn from(val: NonEmptyVec<T>) -> Self {
        val.vec
    }
}

impl<T> Deref for NonEmptySlice<'_, T> {
    type Target = [T];

    /// Dereferences to the underlying slice.
    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<'s, T> TryFrom<&'s [T]> for NonEmptySlice<'s, T> {
    type Error = NonEmptyError;

    /// Tries to convert a `&[T]` into a `NonEmptySlice<T>`, erroring if the slice is empty.
    fn try_from(slice: &'s [T]) -> Result<Self, Self::Error> {
        if slice.is_empty() {
            return Err(NonEmptyError::Empty);
        }

        // Safety: The slice must be non-empty.
        Ok(unsafe { Self::new_unchecked(slice) })
    }
}

impl<'s, T> From<NonEmptySlice<'s, T>> for &'s [T] {
    /// Converts a `NonEmptySlice<T>` into a `&[T]`.
    fn from(slice: NonEmptySlice<'s, T>) -> Self {
        slice.slice
    }
}

impl<'s, T> From<&'s [T; 1]> for NonEmptySlice<'s, T> {
    /// Converts a single-element slice into a `NonEmptySlice<T>`.
    fn from(slice: &'s [T; 1]) -> Self {
        NonEmptySlice { slice }
    }
}

impl<T> From<NonEmptySlice<'_, T>> for Vec<T>
where
    T: Clone,
{
    /// Converts a `NonEmptySlice<T>` into an owned `Vec<T>`, cloning the elements.
    fn from(slice: NonEmptySlice<'_, T>) -> Self {
        slice.slice.to_vec()
    }
}

impl<T> Clone for NonEmptySlice<'_, T> {
    /// Clones a `NonEmptySlice<T>` by copying a reference to the slice.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for NonEmptySlice<'_, T> {}
