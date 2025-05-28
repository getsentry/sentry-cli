#![warn(clippy::allow_attributes)]
#![warn(clippy::unnecessary_wraps)]

mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    commands::main()
}
