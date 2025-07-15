mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    if cfg!(target_os = "linux") {
        panic!("This is a test");
    }

    commands::main()
}
