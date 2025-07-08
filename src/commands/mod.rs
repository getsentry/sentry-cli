//! This module implements the root command of the CLI tool.

use anyhow::Result;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Generator, Shell};
use log::{debug, info, set_logger, set_max_level, LevelFilter};
use std::borrow::Cow;
use std::io;
use std::process;
use std::{env, iter};

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::constants::{ARCH, PLATFORM, VERSION};
use crate::utils::auth_token::{redact_token_from_string, AuthToken};
use crate::utils::logging::set_quiet_mode;
use crate::utils::logging::Logger;
use crate::utils::system::{load_dotenv, print_error, set_panic_hook, QuietExit};
use crate::utils::update::run_sentrycli_update_nagger;
use crate::utils::value_parsers::auth_token_parser;

mod derive_parser;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(bash_hook);
        $mac!(debug_files);
        $mac!(deploys);
        $mac!(events);
        $mac!(files);
        $mac!(info);
        $mac!(issues);
        $mac!(login);
        #[cfg(feature = "unstable-mobile-app")]
        $mac!(mobile_app);
        $mac!(monitors);
        $mac!(organizations);
        $mac!(projects);
        $mac!(react_native);
        $mac!(releases);
        $mac!(repos);
        $mac!(send_event);
        $mac!(send_envelope);
        $mac!(send_metric);
        $mac!(sourcemaps);
        #[cfg(not(feature = "managed"))]
        $mac!(uninstall);
        #[cfg(not(feature = "managed"))]
        $mac!(update);
        $mac!(upload_dif);
        $mac!(upload_dsym);
        $mac!(upload_proguard);
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
    "debug-files",
    "deploys",
    "events",
    "files",
    "info",
    "issues",
    "login",
    "organizations",
    "projects",
    "releases",
    "repos",
    "sourcemaps",
];

/// The long auth token argument (--auth-token).
const AUTH_TOKEN_ARG: &str = "auth-token";

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn preexecute_hooks() -> Result<bool> {
    return sentry_react_native_xcode_wrap();

    #[cfg(target_os = "macos")]
    fn sentry_react_native_xcode_wrap() -> Result<bool> {
        if let Ok(val) = env::var("__SENTRY_RN_WRAP_XCODE_CALL") {
            if &val == "1" {
                crate::commands::react_native::xcode::wrap_call()?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    // This function needs to return Result<bool> to remain compatible with the
    // macOS implementation.
    #[expect(clippy::unnecessary_wraps)]
    #[cfg(not(target_os = "macos"))]
    fn sentry_react_native_xcode_wrap() -> Result<bool> {
        Ok(false)
    }
}

fn configure_args(config: &mut Config, matches: &ArgMatches) {
    if let Some(api_key) = matches.get_one::<String>("api_key") {
        config.set_auth(Auth::Key(api_key.to_owned()));
    }

    if let Some(auth_token) = matches.get_one::<AuthToken>("auth_token") {
        config.set_auth(Auth::Token(auth_token.to_owned()));
    }

    if let Some(url) = matches.get_one::<String>("url") {
        config.set_base_url(url);
    }

    if let Some(headers) = matches.get_many::<String>("headers") {
        let headers = headers.map(|h| h.to_owned()).collect();
        config.set_headers(headers);
    }
}

fn app() -> Command {
    Command::new("sentry-cli")
        .version(VERSION)
        .about(ABOUT)
        .max_term_width(100)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("url").value_name("URL").long("url").help(
            "Fully qualified URL to the Sentry server.{n}\
             [default: https://sentry.io/]",
        ))
        .arg(
            Arg::new("headers")
                .long("header")
                .value_name("KEY:VALUE")
                .action(ArgAction::Append)
                .global(true)
                .help(
                    "Custom headers that should be attached to all requests{n}in key:value format.",
                ),
        )
        .arg(
            Arg::new("auth_token")
                .value_name("AUTH_TOKEN")
                .long(AUTH_TOKEN_ARG)
                .global(true)
                .value_parser(auth_token_parser)
                .help("Use the given Sentry auth token."),
        )
        .arg(
            Arg::new("api_key")
                .value_name("API_KEY")
                .long("api-key")
                .help("Use the given Sentry API key."),
        )
        .arg(
            Arg::new("log_level")
                .value_name("LOG_LEVEL")
                .long("log-level")
                .value_parser(value_parser!(LevelFilter))
                .ignore_case(true)
                .global(true)
                .help("Set the log output verbosity. [possible values: trace, debug, info, warn, error]"),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .visible_alias("silent")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Do not print any output while preserving correct exit code. This flag is currently implemented only for selected subcommands."),
        )
        .arg(
          Arg::new("allow_failure")
              .long("allow-failure")
              .action(ArgAction::SetTrue)
              .global(true)
              .hide(true)
              .help("Always return 0 exit code."),
        )
        .subcommand(
            Command::new("completions")
            .about("Generate completions for the specified shell.")
            .arg_required_else_help(true)
            .arg(
                Arg::new("shell")
                    .help("The shell to print completions for.")
                    .value_parser(value_parser!(Shell)),
            )
        )
}

fn add_commands(mut app: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            let cmd = $name::make_command(Command::new(stringify!($name).replace("_", "-")));
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

    let mut cmd = app();
    cmd = add_commands(cmd);
    let matches = cmd.get_matches();
    let log_level = matches.get_one::<LevelFilter>("log_level");
    if let Some(&log_level) = log_level {
        set_max_level(log_level);
    }
    let mut config = Config::from_cli_config()?;
    configure_args(&mut config, &matches);
    set_quiet_mode(matches.get_flag("quiet"));

    if let Some(&log_level) = log_level {
        config.set_log_level(log_level);
    }

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
            // Check whether the previous argument is "--auth-token"
            .zip(
                iter::once(false)
                    .chain(env::args().map(|arg| arg == format!("--{AUTH_TOKEN_ARG}")))
            )
            .map(|(a, is_auth_token_arg)| {
                let redact_replacement = "[REDACTED]";

                // Redact anything that comes after --auth-token
                let redacted = if is_auth_token_arg {
                    Cow::Borrowed(redact_replacement)
                } else if a.starts_with(&format!("--{AUTH_TOKEN_ARG}=")) {
                    Cow::Owned(format!("--{AUTH_TOKEN_ARG}={redact_replacement}"))
                } else {
                    redact_token_from_string(&a, redact_replacement)
                };

                format!("\"{redacted}\"")
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    if let Some(argmatches) = matches.subcommand_matches("completions") {
        let mut cmd = app();
        cmd = add_commands(cmd);
        if let Some(generator) = argmatches.get_one::<Shell>("shell") {
            eprintln!("Generating completion file for {generator}...");
            print_completions(*generator, &mut cmd);
            return Ok(());
        }
    }

    match run_command(&matches) {
        Ok(()) => Ok(()),
        Err(e) => {
            if Config::current().get_allow_failure(&matches) {
                print_error(&e);
                eprintln!("\nCommand failed, however, \"SENTRY_ALLOW_FAILURE\" variable or \"allow-failure\" flag was set. Exiting with 0 exit code.");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn setup() {
    set_panic_hook();

    // Store the result of loading the dotenv file. We must load the dotenv file
    // before setting the log level, as the log level can be set in the dotenv
    // file, but we should only log a warning after setting the log level.
    let load_dotenv_result = load_dotenv();

    // we use debug internally but our log handler then rejects to a lower limit.
    // This is okay for our uses but not as efficient.
    set_max_level(LevelFilter::Debug);
    #[expect(clippy::unwrap_used, reason = "legacy code")]
    set_logger(&Logger).unwrap();

    if let Err(e) = load_dotenv_result {
        log::warn!("Failed to load .env file: {}", e);
    }
}

/// Executes the command line application and exits the process.
pub fn main() -> ! {
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
