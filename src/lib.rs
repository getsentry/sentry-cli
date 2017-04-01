//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "1024"]

#[cfg(not(windows))] #[macro_use] extern crate chan as chan_crate;
#[macro_use] extern crate error_chain as error_chain_crate;
#[macro_use] extern crate serde_derive as serde_derive_crate;
#[macro_use] extern crate log as log_crate;
#[macro_use] extern crate if_chain as if_chain_crate;
#[macro_use] extern crate lazy_static as lazy_static_crate;

mod crates {
    #[cfg(not(windows))]
    pub mod chan { pub use chan_crate::*; }

    #[cfg(not(windows))] pub extern crate chan_signal;
    pub extern crate curl;
    pub extern crate clap;
    pub extern crate ini;
    pub extern crate backtrace;
    pub extern crate itertools;
    pub extern crate serde;
    pub extern crate serde_json;
    pub extern crate url;
    pub extern crate uuid;
    pub extern crate walkdir;
    pub extern crate which;
    pub extern crate zip;
    pub extern crate sha1;
    pub extern crate sourcemap;
    pub extern crate open;
    pub extern crate runas;
    pub extern crate term;
    pub extern crate plist;
    pub extern crate might_be_minified;
    pub mod log { pub use log_crate::*; }
    pub extern crate chrono;
    pub extern crate regex;
    pub extern crate pbr;
    #[cfg(target_os="macos")]
    pub extern crate osascript;
    #[cfg(target_os="macos")]
    pub extern crate unix_daemonize;
    pub extern crate dotenv;
    #[cfg(not(windows))]
    pub extern crate openssl_probe;
    pub extern crate elementtree;
    pub extern crate prettytable;
}

mod macros;

pub mod prelude;
pub mod api;
pub mod commands;
pub mod event;
pub mod errors;
pub mod config;
pub mod utils;
pub mod macho;
pub mod xcode;
pub mod gradle;
pub mod sourcemaputils;
pub mod constants;

use std::io::Write;

#[cfg(not(windows))]
fn real_main() {
    use crates::openssl_probe::init_ssl_cert_env_vars;
    init_ssl_cert_env_vars();
    if let Some(signal) = utils::run_or_interrupt(commands::main) {
        use crates::chan_signal::Signal;
        if signal == Signal::INT {
            println!("Interrupted!");
        }
    }
}

#[cfg(windows)]
fn real_main() {
    commands::main();
}

fn init_backtrace() {
    use crates::backtrace::Backtrace;
    use std::panic;
    use std::thread;

    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::new();

        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match info.payload().downcast_ref::<String>() {
                    Some(s) => &**s,
                    None => "Box<Any>",
                }
            }
        };

        match info.location() {
            Some(location) => {
                println_stderr!("thread '{}' panicked at '{}': {}:{}\n\n{:?}",
                         thread,
                         msg,
                         location.file(),
                         location.line(),
                         backtrace);
            }
            None => println_stderr!("thread '{}' panicked at '{}'{:?}", thread, msg, backtrace),
        }
    }));
}

/// Executes the command line application and exits the process.
pub fn main() {
    crates::dotenv::dotenv().ok();
    init_backtrace();
    real_main();
}
