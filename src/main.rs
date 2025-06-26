mod api;
mod commands;
mod config;
mod constants;
mod utils;

pub fn main() -> ! {
    #[cfg(not(target_os = "linux"))]
    let _: &str = 2;
    commands::main()
}
