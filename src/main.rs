#![warn(clippy::allow_attributes)]
#![warn(clippy::unnecessary_wraps)]
#![warn(clippy::unwrap_used)]

mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    commands::main()
}
