#![macro_use]

macro_rules! fail {
    ($expr:expr) => (
        return Err(::std::convert::From::from($expr));
    )
}

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
