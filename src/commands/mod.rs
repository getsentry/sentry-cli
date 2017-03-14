//! This module implements the root command of the CLI tool.

use std::io;
use std::io::Write;
use std::env;
use std::process;

use log;
use clap::{Arg, App, AppSettings};

use prelude::*;
use constants::VERSION;
use utils::Logger;
use config::{Config, Auth};

const ABOUT: &'static str = "
Command line utility for Sentry.

This tool helps you managing remote resources on a Sentry server like
sourcemaps, debug symbols, releases or similar.  Use `--help` on the
subcommands to learn more about them.";


macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dsym);
        $mac!(releases);
        $mac!(issues);
        $mac!(update);
        $mac!(uninstall);
        $mac!(info);
        $mac!(login);
        $mac!(send_event);
        #[cfg(target_os="macos")]
        $mac!(react_native_xcode);
    }
}

// it would be great if this could be a macro expansion as well
// but rust bug #37663 breaks location information then.
pub mod upload_dsym;
pub mod releases;
pub mod issues;
pub mod update;
pub mod uninstall;
pub mod info;
pub mod login;
pub mod send_event;

#[cfg(target_os="macos")]
pub mod react_native_xcode;

fn preexecute_hooks() -> Result<bool> {
    return sentry_react_native_xcode_wrap();

    #[cfg(target_os="macos")]
    fn sentry_react_native_xcode_wrap() -> Result<bool> {
        if let Ok(val) = env::var("__SENTRY_RN_WRAP_XCODE_CALL") {
            env::remove_var("__SENTRY_RN_WRAP_XCODE_CALL");
            if &val == "1" {
                react_native_xcode::wrap_call()?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    #[cfg(not(target_os="macos"))]
    fn sentry_react_native_xcode_wrap() -> Result<bool> {
        Ok(false)
    }
}

/// Given an argument vector and a `Config` this executes the
/// command line and returns the result.
pub fn execute(args: Vec<String>, config: &mut Config) -> Result<()> {
    // special case for the xcode integration for react native.  For more
    // information see commands/react_native_xcode.rs
    if preexecute_hooks()? {
        return Ok(());
    }

    let mut app = App::new("sentry-cli")
        .version(VERSION)
        .about(ABOUT)
        .max_term_width(100)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("url")
            .value_name("URL")
            .long("url")
            .help("The sentry API URL{n}defaults to https://sentry.io/"))
        .arg(Arg::with_name("auth_token")
            .value_name("AUTH_TOKEN")
            .long("auth-token")
            .help("The sentry auth token to use"))
        .arg(Arg::with_name("api_key")
            .value_name("API_KEY")
            .long("api-key")
            .help("The sentry API key to use"))
        .arg(Arg::with_name("log_level")
            .value_name("LOG_LEVEL")
            .long("log-level")
            .help("The log level for sentry-cli{n}\
                   (valid levels: TRACE, DEBUG, INFO, WARN, ERROR)"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand($name::make_app(
                App::new(stringify!($name).replace("_", "-").as_str())));
        }}
    }
    each_subcommand!(add_subcommand);

    let matches = app.get_matches_from_safe(args)?;

    if let Some(url) = matches.value_of("url") {
        config.url = url.to_owned();
    }
    if let Some(api_key) = matches.value_of("api_key") {
        config.auth = Some(Auth::Key(api_key.to_owned()));
    }
    if let Some(auth_token) = matches.value_of("auth_token") {
        config.auth = Some(Auth::Token(auth_token.to_owned()));
    }
    if let Some(level_str) = matches.value_of("log_level") {
        match level_str.parse() {
            Ok(level) => {
                config.log_level = level;
            }
            Err(_) => {
                fail!("Unknown log level: {}", level_str);
            }
        }
    }

    log::set_logger(|max_log_level| {
            max_log_level.set(config.log_level);
            Box::new(Logger)
        })
        .ok();

    macro_rules! execute_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            if let Some(sub_matches) = matches.subcommand_matches(cmd) {
                return Ok($name::execute(&sub_matches, &config)?);
            }
        }}
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}

fn run() -> Result<()> {
    execute(env::args().collect(), &mut Config::from_cli_config()?)
}

/// Executes the command line application and exists the process.
pub fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(err) => {
            if let &ErrorKind::Clap(ref clap_err) = err.kind() {
                clap_err.exit();
            }

            writeln!(&mut io::stderr(), "error: {}", err).ok();

            if env::var("RUST_BACKTRACE") == Ok("1".into()) {
                writeln!(&mut io::stderr(), "").ok();
                writeln!(&mut io::stderr(), "{:?}", err.backtrace()).ok();
            }

            process::exit(1);
        }
    }
}
