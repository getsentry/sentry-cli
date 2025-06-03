#![warn(clippy::allow_attributes)]
#![warn(clippy::unnecessary_wraps)]
#![cfg_attr(
    not(test),
    warn(
        clippy::unwrap_used,
        reason = "unwrap only allowed in tests. Please return a result, or use expect, instead."
    )
)]

mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    commands::main()
}
