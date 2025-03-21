#![warn(clippy::unnecessary_wraps)]

mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    commands::main();
    println!("This is unreachable, and should cause a clippy warning");
}
