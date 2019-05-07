use std::borrow::Cow;
use std::env;
use std::process;

use chrono::{DateTime, Utc};
use console::style;
use failure::{Error, Fail};
use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;

#[cfg(not(windows))]
pub fn run_or_interrupt<F>(f: F)
where
    F: FnOnce() -> (),
    F: Send + 'static,
{
    let (tx, rx) = crossbeam_channel::bounded(100);
    let signals =
        signal_hook::iterator::Signals::new(&[signal_hook::SIGTERM, signal_hook::SIGINT]).unwrap();

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
        if signal == signal_hook::SIGINT {
            eprintln!("Interrupted!");
        }
    }
}

#[cfg(windows)]
pub fn run_or_interrupt<F>(f: F)
where
    F: FnOnce() -> (),
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
fn is_homebrew_install_result() -> Result<bool, Error> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.pop();
    exe.set_file_name("INSTALL_RECEIPT.json");
    Ok(exe.is_file())
}

#[cfg(windows)]
fn is_homebrew_install_result() -> Result<bool, Error> {
    Ok(false)
}

fn is_npm_install_result() -> Result<bool, Error> {
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

/// Expands environment variables in a string
pub fn expand_envvars(s: &str) -> Cow<'_, str> {
    expand_vars(s, |key| env::var(key).unwrap_or_else(|_| "".to_string()))
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
    if let Some(ref clap_err) = err.downcast_ref::<clap::Error>() {
        clap_err.exit();
    }

    for (idx, cause) in err.iter_chain().enumerate() {
        match idx {
            0 => eprintln!("{} {}", style("error:").red(), cause),
            _ => eprintln!("  {} {}", style("caused by:").dim(), cause),
        }
    }

    if Config::current().get_log_level() < log::LevelFilter::Info {
        eprintln!();
        eprintln!("{}", style("Add --log-level=[info|debug] or export SENTRY_LOG_LEVEL=[info|debug] to see more output.").dim());
        eprintln!(
            "{}",
            style("Please attach the full debug log to all bug reports.").dim()
        );
    }

    if env::var("RUST_BACKTRACE") == Ok("1".into()) {
        eprintln!();
        let backtrace = format!("{:?}", err.backtrace());
        eprintln!("{}", style(&backtrace).dim());
    }
}

/// Given a system time returns the unix timestamp as f64
pub fn to_timestamp(tm: DateTime<Utc>) -> f64 {
    tm.timestamp() as f64
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
            None => eprintln!("thread '{}' panicked at '{}'{:?}", thread, msg, backtrace),
        }

        #[cfg(feature = "with_client_implementation")]
        {
            crate::utils::crashreporting::flush_events();
        }
    }));
}

#[cfg(target_os = "macos")]
pub fn get_model() -> Option<String> {
    if let Some(model) = Config::current().get_model() {
        return Some(model);
    }

    use std::ptr;

    unsafe {
        let mut size = 0;
        libc::sysctlbyname(
            "hw.model\x00".as_ptr() as *const i8,
            ptr::null_mut(),
            &mut size,
            ptr::null_mut(),
            0,
        );
        let mut buf = vec![0u8; size as usize];
        libc::sysctlbyname(
            "hw.model\x00".as_ptr() as *const i8,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        Some(String::from_utf8_lossy(&buf).to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn get_family() -> Option<String> {
    if let Some(family) = Config::current().get_family() {
        return Some(family);
    }

    use if_chain::if_chain;

    lazy_static! {
        static ref FAMILY_RE: Regex = Regex::new(r#"([a-zA-Z]+)\d"#).unwrap();
    }

    if_chain! {
        if let Some(model) = get_model();
        if let Some(m) = FAMILY_RE.captures(&model);
        if let Some(group) = m.get(1);
        then {
            Some(group.as_str().to_string())
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_model() -> Option<String> {
    Config::current().get_model()
}

#[cfg(not(target_os = "macos"))]
pub fn get_family() -> Option<String> {
    Config::current().get_family()
}

/// Indicates that sentry-cli should quit without printing anything.
#[derive(Fail, Debug)]
#[fail(display = "sentry-cli exit with {}", _0)]
pub struct QuietExit(pub i32);

/// Loads a .env file
pub fn load_dotenv() {
    if env::var("SENTRY_LOAD_DOTENV")
        .map(|x| x.as_str() == "1")
        .unwrap_or(true)
    {
        dotenv::dotenv().ok();
    }
}
