use std::any::Any;
use std::backtrace::Backtrace;
use std::borrow::Cow;
use std::panic::{self, Location, PanicHookInfo};
#[cfg(target_os = "macos")]
use std::process;
use std::{env, thread};

use anyhow::{Context as _, Error, Result};
use console::style;
use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;

/// Propagate an exit status outwards.
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

fn is_npm_install_result() -> Result<bool> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.set_file_name("package.json");
    Ok(exe.is_file())
}

/// Checks if we were installed from homebrew
pub fn is_homebrew_install() -> bool {
    #[cfg(not(windows))]
    if let Ok(mut exe) = env::current_exe().and_then(|p| p.canonicalize()) {
        exe.pop();
        exe.set_file_name("INSTALL_RECEIPT.json");
        return exe.is_file();
    }

    false
}

/// Checks if we were installed via npm
pub fn is_npm_install() -> bool {
    is_npm_install_result().unwrap_or(false)
}

/// Expands variables in a string
pub fn expand_vars<F: Fn(&str) -> String>(s: &str, f: F) -> Cow<'_, str> {
    lazy_static! {
        static ref VAR_RE: Regex =
            Regex::new(r"\$(\$|[a-zA-Z0-9_]+|\([^)]+\)|\{[^}]+\})").expect("this regex is valid");
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

    // Debug style for error includes cause chain and backtrace (if available).
    eprintln!("{} {:?}", style("error:").red(), err);

    if Config::current_opt().is_none_or(|config| config.get_log_level() < log::LevelFilter::Info) {
        eprintln!();
        eprintln!("{}", style("Add --log-level=[info|debug] or export SENTRY_LOG_LEVEL=[info|debug] to see more output.").dim());
        eprintln!(
            "{}",
            style("Please attach the full debug log to all bug reports.").dim()
        );
    }
}

/// Sets the panic hook to use our custom panic hook.
///
/// See [panic_hook] for more details on how the custom panic hook behaves.
pub fn set_panic_hook() {
    panic::set_hook(Box::new(panic_hook));
}

/// Indicates that sentry-cli should quit without printing anything.
#[derive(thiserror::Error, Debug)]
#[error("sentry-cli exit with {0}")]
pub struct QuietExit(pub i32);

/// Loads a .env file
pub fn load_dotenv() -> Result<()> {
    let load_dotenv_unset = env::var("SENTRY_LOAD_DOTENV")
        .map(|x| x.as_str() != "1")
        .unwrap_or(false);

    if load_dotenv_unset {
        return Ok(());
    }

    let custom_dotenv_paths: &[_] = if let Ok(path) = env::var("SENTRY_DOTENV_PATH") {
        &[path]
    } else if let Ok(paths) = env::var("SENTRY_DOTENV_PATHS") {
        &paths
            .split(",")
            .map(|path| path.trim())
            .filter(|path| !path.is_empty())
            .map(|path| path.to_string())
            .collect::<Vec<_>>()
    } else {
        // Fallback to default dotenv
        dotenvy::dotenv()
            .map_or_else(|e| if e.not_found() { Ok(()) } else { Err(e) }, |_| Ok(()))
            .context("We found a .env file, but failed to load it.")?;
        return Ok(());
    };

    for path in custom_dotenv_paths {
        dotenvy::from_path_override(path)
            .with_context(|| format!("Failed to load custom .env file: {path}"))?;
    }

    Ok(())
}

/// Custom panic hook for Sentry CLI
///
/// This custom panic hook captures a more user-friendly panic message, which indicates
/// that the panic is an internal error in the Sentry CLI, and which directs users to
/// open a bug report issue when encountering a panic.
///
/// The panic captures and prints a backtrace, regardless of whether the RUST_BACKTRACE
/// environment variable is set.
fn panic_hook(info: &PanicHookInfo) {
    const PANIC_MESSAGE: &str = "Uh-oh! 😬 Sentry CLI has just crashed due to an internal error. \
        Please open a bug report issue at https://github.com/getsentry/sentry-cli/issues/new?template=BUG_REPORT.yml. 🐞";

    eprintln!(
        "{}\n\n{}\n\n{}",
        console::style("🔥 Internal Error in Sentry CLI 🔥")
            .bold()
            .red(),
        PANIC_MESSAGE,
        display_technical_details(info, &Backtrace::force_capture())
    );
}

/// Generates the "technical details" section of the panic message
fn display_technical_details(info: &PanicHookInfo, backtrace: &Backtrace) -> String {
    format!(
        "🔬 Technical Details 🔬\n\n{} panicked at {}:\n{}\n\nStack Backtrace:\n{}",
        display_thread_details(),
        display_panic_location(info.location()),
        display_panic_payload(info.payload()),
        backtrace
    )
}

/// Formats the current thread name for display in the panic message
fn display_thread_details() -> String {
    match thread::current().name() {
        Some(name) => format!("thread '{name}'"),
        None => "unknown thread".into(),
    }
}

/// Formats the panic location for display in the panic message
fn display_panic_location(location: Option<&Location>) -> String {
    if let Some(location) = location {
        location.to_string()
    } else {
        "unknown location".into()
    }
}

/// Formats the panic payload for display in the panic message
fn display_panic_payload(payload: &dyn Any) -> &str {
    if let Some(&payload) = payload.downcast_ref() {
        payload
    } else if let Some(payload) = payload.downcast_ref::<String>() {
        payload.as_str()
    } else {
        ""
    }
}
