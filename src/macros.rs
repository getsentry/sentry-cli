//! This module provides some useful internal macros.

#![macro_use]

/// A helper macro that fails the current function with a given error.
///
/// This goes through `From::from` the same way how `try!` operates and is
/// a nice shorthand that makes code more concise in particular because
/// types do not have to directly match the return value.
macro_rules! fail {
    ($expr:expr) => (
        return Err(::std::convert::From::from($expr));
    );
    ($expr:expr $(, $more:expr)+) => (
        fail!(format!($expr, $($more),*))
    )
}

/// A version of `try!` that works within iterators.  In particular it
/// wraps the returned error in `Some(...)`.
macro_rules! iter_try {
    ($expr:expr) => {
        match $expr {
            Ok(rv) => rv,
            Err(err) => {
                return Some(Err(::std::convert::From::from(err)));
            }
        }
    }
}
