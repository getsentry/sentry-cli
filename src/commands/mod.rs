//! This module implements the root command of the CLI tool.

use std::io;
use std::io::Write;
use std::env;
use std::process;

use log;
use clap::{Arg, App, AppSettings};

use prelude::*;
use constants::VERSION;
use utils::{make_subcommand, Logger};
use config::{Config, Auth};


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
    }
}

macro_rules! import_subcommand {
    ($name:ident) => { pub mod $name; }
}
each_subcommand!(import_subcommand);

/// Given an argument vector and a `Config` this executes the
/// command line and returns the result.
pub fn execute(args: Vec<String>, config: &mut Config) -> Result<()> {
    let mut app = App::new("sentry-cli")
        .version(VERSION)
        .about("Command line utility for Sentry")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("url")
            .value_name("URL")
            .long("url")
            .help("The sentry API URL"))
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
            .help("The log level for the sentrycli"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand($name::make_app(
                make_subcommand(&stringify!($name).replace("_", "-"))));
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
            if let &ErrorKind::ClapError(ref clap_err) = err.kind() {
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
