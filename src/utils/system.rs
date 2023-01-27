use std::borrow::Cow;
use std::env;
use std::process;

use anyhow::{Error, Result};
use console::style;
use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;
#[cfg(not(windows))]
use crate::utils::xcode::launched_from_xcode;

#[cfg(not(windows))]
pub fn run_or_interrupt<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    // See: https://github.com/getsentry/sentry-cli/pull/1104
    if launched_from_xcode() {
        f();
        return;
    }

    let (tx, rx) = crossbeam_channel::bounded(100);
    let mut signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ])
    .unwrap();

    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            f();
            tx.send(0).ok();
        });
    }

    std::thread::spawn(move || {
        for signal in signals.forever() {
            tx.send(signal).ok();
        }
    });

    if let Ok(signal) = rx.recv() {
        if signal == signal_hook::consts::SIGINT {
            eprintln!("Interrupted!");
        }
    }
}

#[cfg(windows)]
pub fn run_or_interrupt<F>(f: F)
where
    F: FnOnce(),
    F: Send + 'static,
{
    f();
}

/// Propagate an exit status outwarts
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

    if Config::current().get_log_level() < log::LevelFilter::Info {
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
pub fn load_dotenv() {
    if env::var("SENTRY_LOAD_DOTENV")
        .map(|x| x.as_str() == "1")
        .unwrap_or(true)
    {
        if let Ok(path) = env::var("SENTRY_DOTENV_PATH") {
            dotenv::from_path(path).ok();
        } else {
            dotenv::dotenv().ok();
        }
    }
}
