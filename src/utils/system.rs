use std::borrow::Cow;
use std::env;
use std::process;

use config::Config;

#[cfg(not(windows))]
use chan_signal::{notify, Signal};
use chrono::{DateTime, Utc};
use clap;
use console::style;
use dotenv;
use failure::Error;
use regex::{Captures, Regex};

#[cfg(not(windows))]
pub fn run_or_interrupt<F>(f: F)
where
    F: FnOnce() -> (),
    F: Send + 'static,
{
    use chan;
    let run = |_sdone: chan::Sender<()>| f();
    let signal = notify(&[Signal::INT, Signal::TERM]);
    let (sdone, rdone) = chan::sync(0);
    ::std::thread::spawn(move || run(sdone));

    let mut rv = None;

    chan_select! {
        signal.recv() -> signal => { rv = signal; },
        rdone.recv() => {}
    }

    if let Some(signal) = rv {
        use chan_signal::Signal;
        if signal == Signal::INT {
            println!("Interrupted!");
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
    exe.pop();
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
pub fn expand_envvars(s: &str) -> Cow<str> {
    expand_vars(s, |key| env::var(key).unwrap_or_else(|_| "".to_string()))
}

/// Expands variables in a string
pub fn expand_vars<F: Fn(&str) -> String>(s: &str, f: F) -> Cow<str> {
    lazy_static! {
        static ref VAR_RE: Regex = Regex::new(r"\$(\$|[a-zA-Z0-9_]+|\([^)]+\)|\{[^}]+\})").unwrap();
    }
    VAR_RE.replace_all(s, |caps: &Captures| {
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
    use backtrace::Backtrace;
    use std::panic;
    use std::thread;

    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::new();

        let thread = thread::current();
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
            use utils::crashreporting::flush_events;
            flush_events();
        }
    }));
}

#[cfg(target_os = "macos")]
pub fn get_model() -> Option<String> {
    if let Some(model) = Config::get_current().get_model() {
        return Some(model);
    }

    use libc;
    use libc::c_void;
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
            buf.as_mut_ptr() as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        Some(String::from_utf8_lossy(&buf).to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn get_family() -> Option<String> {
    if let Some(family) = Config::get_current().get_family() {
        return Some(family);
    }

    use regex::Regex;
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
    Config::get_current().get_model()
}

#[cfg(not(target_os = "macos"))]
pub fn get_family() -> Option<String> {
    Config::get_current().get_family()
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
