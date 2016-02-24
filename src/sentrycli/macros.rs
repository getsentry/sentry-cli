#![macro_use]

macro_rules! fail {
    ($expr:expr) => (
        return Err(::std::convert::From::from($expr));
    );
    ($expr:expr $(, $more:expr)+) => (
        return fail!(format!($expr, $($more),*))
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
