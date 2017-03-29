//! This is the library that powers the `sentry-cli` tool.  The primary
//! exported function is `main` which is directly invoked from the
//! compiled binrary that links against this library.

#![recursion_limit = "1024"]

#[cfg(not(windows))]
#[macro_use]
extern crate chan;
#[cfg(not(windows))]
extern crate chan_signal;
extern crate curl;
extern crate clap;
extern crate ini;
extern crate backtrace;
extern crate itertools;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uuid;
extern crate walkdir;
extern crate which;
extern crate zip;
extern crate sha1;
extern crate sourcemap;
extern crate open;
extern crate runas;
extern crate term;
extern crate plist;
extern crate might_be_minified;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate if_chain;
extern crate chrono;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate pbr;
#[cfg(target_os="macos")]
extern crate osascript;
#[cfg(target_os="macos")]
extern crate unix_daemonize;
extern crate dotenv;
#[cfg(not(windows))]
extern crate openssl_probe;
extern crate elementtree;

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
    use openssl_probe::init_ssl_cert_env_vars;
    init_ssl_cert_env_vars();
    if let Some(signal) = utils::run_or_interrupt(commands::main) {
        use chan_signal::Signal;
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
    use backtrace::Backtrace;
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
    dotenv::dotenv().ok();
    init_backtrace();
    real_main();
}
