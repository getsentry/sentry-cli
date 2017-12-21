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

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        use std::io::Write;
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);
