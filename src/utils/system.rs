use std::borrow::Cow;
use std::env;
#[cfg(target_os = "macos")]
use std::process;

use anyhow::{Error, Result};
use console::style;
use dotenv::Result as DotenvResult;
use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;

/// Propagate an exit status outwarts.
/// We only use this function in the macOS binary.
#[cfg(target_os = "macos")]
pub fn propagate_exit_status(status: process::ExitStatus) {
    if !status.success() {
        if let Some(code) = status.code() {
            process::exit(code);
        } else {
            process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn is_homebrew_install_result() -> Result<bool> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.pop();
    exe.set_file_name("INSTALL_RECEIPT.json");
    Ok(exe.is_file())
}

#[cfg(windows)]
fn is_homebrew_install_result() -> Result<bool> {
    Ok(false)
}

fn is_npm_install_result() -> Result<bool> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.set_file_name("package.json");
    Ok(exe.is_file())
}

/// Checks if we were installed from homebrew
pub fn is_homebrew_install() -> bool {
    is_homebrew_install_result().unwrap_or(false)
}

/// Checks if we were installed via npm
pub fn is_npm_install() -> bool {
    is_npm_install_result().unwrap_or(false)
}

/// Expands variables in a string
pub fn expand_vars<F: Fn(&str) -> String>(s: &str, f: F) -> Cow<'_, str> {
    lazy_static! {
        static ref VAR_RE: Regex = Regex::new(r"\$(\$|[a-zA-Z0-9_]+|\([^)]+\)|\{[^}]+\})").unwrap();
    }
    VAR_RE.replace_all(s, |caps: &Captures<'_>| {
        let key = &caps[1];
        if key == "$" {
            "$".into()
        } else if &key[..1] == "(" || &key[..1] == "{" {
            f(&key[1..key.len() - 1])
        } else {
            f(key)
        }
    })
}

/// Helper that renders an error to stderr.
pub fn print_error(err: &Error) {
    if let Some(clap_err) = err.downcast_ref::<clap::Error>() {
        clap_err.exit();
    }

    eprintln!("{} {}", style("error:").red(), err);
    err.chain()
        .skip(1)
        .for_each(|cause| eprintln!("  {} {}", style("caused by:").dim(), cause));

    if Config::current_opt().map_or(true, |config| {
        config.get_log_level() < log::LevelFilter::Info
    }) {
        eprintln!();
        eprintln!("{}", style("Add --log-level=[info|debug] or export SENTRY_LOG_LEVEL=[info|debug] to see more output.").dim());
        eprintln!(
            "{}",
            style("Please attach the full debug log to all bug reports.").dim()
        );
    }
}

/// Initializes the backtrace support
pub fn init_backtrace() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = backtrace::Backtrace::new();

        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        match info.location() {
            Some(location) => {
                eprintln!(
                    "thread '{}' panicked at '{}': {}:{}\n\n{:?}",
                    thread,
                    msg,
                    location.file(),
                    location.line(),
                    backtrace
                );
            }
            None => eprintln!("thread '{thread}' panicked at '{msg}'{backtrace:?}"),
        }
    }));
}

/// Indicates that sentry-cli should quit without printing anything.
#[derive(thiserror::Error, Debug)]
#[error("sentry-cli exit with {0}")]
pub struct QuietExit(pub i32);

/// Loads a .env file
pub fn load_dotenv() -> DotenvResult<()> {
    let load_dotenv_unset = env::var("SENTRY_LOAD_DOTENV")
        .map(|x| x.as_str() != "1")
        .unwrap_or(false);

    if load_dotenv_unset {
        return Ok(());
    }

    match env::var("SENTRY_DOTENV_PATH") {
        Ok(path) => dotenv::from_path(path),
        Err(_) => dotenv::dotenv().map(|_| ()),
    }
    .map_or_else(
        |error| {
            // We only propagate errors if the .env file was found and failed to load.
            if error.not_found() {
                Ok(())
            } else {
                Err(error)
            }
        },
        |_| Ok(()),
    )
}
