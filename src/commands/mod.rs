//! This module implements the root command of the CLI tool.

use std::env;
use std::process;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use log::{debug, info, set_logger, set_max_level, LevelFilter};

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::constants::{ARCH, PLATFORM, VERSION};
use crate::utils::logging::Logger;
use crate::utils::system::{init_backtrace, load_dotenv, print_error, QuietExit};
use crate::utils::update::run_sentrycli_update_nagger;

// Nested sub-commands
pub mod react_native_appcenter;
pub mod react_native_gradle;
#[cfg(target_os = "macos")]
pub mod react_native_xcode;

pub mod difutil_bundle_sources;
pub mod difutil_check;
pub mod difutil_find;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dif);
        $mac!(upload_proguard);
        $mac!(releases);
        $mac!(issues);
        $mac!(repos);
        $mac!(projects);
        #[cfg(not(feature = "managed"))]
        $mac!(update);
        #[cfg(not(feature = "managed"))]
        $mac!(uninstall);
        $mac!(info);
        $mac!(login);
        $mac!(send_event);
        $mac!(react_native);
        $mac!(difutil);
    };
}

macro_rules! import_subcommand {
    ($name:ident) => {
        pub mod $name;
    };
}

each_subcommand!(import_subcommand);

const ABOUT: &str = "
Command line utility for Sentry.

This tool helps you manage remote resources on a Sentry server like
sourcemaps, debug symbols or releases.  Use `--help` on the subcommands
to learn more about them.";

// Commands we want to run the update nagger on
const UPDATE_NAGGER_CMDS: &[&str] = &[
    "releases", "issues", "repos", "projects", "info", "login", "difutil",
];

fn preexecute_hooks() -> Result<bool> {
    return sentry_react_native_xcode_wrap();

    #[cfg(target_os = "macos")]
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

    #[cfg(not(target_os = "macos"))]
    fn sentry_react_native_xcode_wrap() -> Result<bool> {
        Ok(false)
    }
}

fn configure_args(config: &mut Config, matches: &ArgMatches) -> Result<()> {
    if let Some(url) = matches.value_of("url") {
        config.set_base_url(url);
    }

    if let Some(headers) = matches.values_of("headers") {
        let headers = headers.map(|h| h.to_owned()).collect();
        config.set_headers(headers);
    }

    if let Some(api_key) = matches.value_of("api_key") {
        config.set_auth(Auth::Key(api_key.to_owned()));
    }

    if let Some(auth_token) = matches.value_of("auth_token") {
        config.set_auth(Auth::Token(auth_token.to_owned()));
    }

    if let Some(level_str) = matches.value_of("log_level") {
        match level_str.parse() {
            Ok(level) => {
                config.set_log_level(level);
            }
            Err(_) => {
                bail!("Unknown log level: {}", level_str);
            }
        }
    }

    Ok(())
}

fn app() -> Command<'static> {
    Command::new("sentry-cli")
        .version(VERSION)
        .about(ABOUT)
        .max_term_width(100)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("url").value_name("URL").long("url").help(
            "Fully qualified URL to the Sentry server.{n}\
             [defaults to https://sentry.io/]",
        ))
        .arg(
            Arg::new("headers")
                .long("header")
                .value_name("KEY:VALUE")
                .multiple_occurrences(true)
                .help(
                    "Custom headers that should be attached to all requests in key:value format.",
                ),
        )
        .arg(
            Arg::new("auth_token")
                .value_name("AUTH_TOKEN")
                .long("auth-token")
                .help("Use the given Sentry auth token."),
        )
        .arg(
            Arg::new("api_key")
                .value_name("API_KEY")
                .long("api-key")
                .help("The given Sentry API key."),
        )
        .arg(
            Arg::new("log_level")
                .value_name("LOG_LEVEL")
                .long("log-level")
                .possible_values(&["trace", "debug", "info", "warn", "error"])
                .ignore_case(true)
                .global(true)
                .help("Set the log output verbosity."),
        )
}

fn add_commands(mut app: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            let cmd = $name::make_app(Command::new(stringify!($name).replace("_", "-").as_str()));
            app = app.subcommand(cmd);
        }};
    }

    each_subcommand!(add_subcommand);
    app
}

fn run_command(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            if let Some(sub_matches) = matches.subcommand_matches(&cmd) {
                let rv = $name::execute(&sub_matches)?;
                if UPDATE_NAGGER_CMDS.iter().any(|x| x == &cmd) {
                    run_sentrycli_update_nagger();
                }
                return Ok(rv);
            }
        }};
    }

    each_subcommand!(execute_subcommand);
    unreachable!();
}

pub fn execute() -> Result<()> {
    // special case for the xcode integration for react native.  For more
    // information see commands/react_native_xcode.rs
    if preexecute_hooks()? {
        return Ok(());
    }

    let mut config = Config::from_cli_config()?;
    let mut app = app();
    app = add_commands(app);
    let matches = app.get_matches();
    configure_args(&mut config, &matches)?;

    // bind the config to the process and fetch an immutable reference to it
    config.bind_to_process();
    if Config::current().get_filename().exists() {
        info!(
            "Loaded config from {}",
            Config::current().get_filename().display()
        );
    }

    debug!(
        "sentry-cli version: {}, platform: \"{}\", architecture: \"{}\"",
        VERSION, PLATFORM, ARCH
    );

    info!(
        "sentry-cli was invoked with the following command line: {}",
        env::args()
            .map(|a| format!("\"{}\"", a))
            .collect::<Vec<String>>()
            .join(" ")
    );

    run_command(&matches)
}

fn setup() {
    init_backtrace();
    load_dotenv();

    // we use debug internally but our log handler then rejects to a lower limit.
    // This is okay for our uses but not as efficient.
    set_max_level(LevelFilter::Debug);
    set_logger(&Logger).unwrap();
}

/// Executes the command line application and exits the process.
pub fn main() {
    setup();

    let exit_code = match execute() {
        Ok(()) => 0,
        Err(err) => {
            let code = if let Some(&QuietExit(code)) = err.downcast_ref() {
                code
            } else {
                print_error(&err);
                1
            };

            // if the user hit an error, it might be time to run the update
            // nagger because maybe they tried to do something only newer
            // versions support.
            run_sentrycli_update_nagger();

            code
        }
    };

    // before we shut down we unbind the api to give the connection pool
    // a chance to collect.  Not doing so has shown to cause hung threads
    // on windows.
    Api::dispose_pool();
    process::exit(exit_code);
}

#[test]
fn verify_app() {
    app().debug_assert();
}
