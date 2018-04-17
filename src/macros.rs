//! This module provides some useful internal macros.

#![macro_use]

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        use std::io::Write;
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);
