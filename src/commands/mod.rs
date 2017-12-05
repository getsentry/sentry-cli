//! This module implements the root command of the CLI tool.

use std::env;
use std::process;

use log;
use clap::{Arg, App, AppSettings};

use prelude::*;
use constants::VERSION;
use utils::{Logger, print_error, run_sentrycli_update_nagger};
use config::{Config, Auth, prepare_environment};

const ABOUT: &'static str = "
Command line utility for Sentry.

This tool helps you managing remote resources on a Sentry server like
sourcemaps, debug symbols or releases.  Use `--help` on the subcommands
to learn more about them.";


macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dsym);
        $mac!(upload_proguard);
        $mac!(releases);
        $mac!(issues);
        $mac!(repos);
        $mac!(projects);
        #[cfg(not(feature="managed"))]
        $mac!(update);
        #[cfg(not(feature="managed"))]
        $mac!(uninstall);
        $mac!(info);
        $mac!(login);
        $mac!(send_event);
        $mac!(react_native);
        $mac!(difutil);
        $mac!(bash_hook);

        // these here exist for legacy reasons only.  They were moved
        // to subcommands of the react-native command.  Note that
        // codepush was never available on that level.
        #[cfg(target_os="macos")]
        $mac!(react_native_xcode);
        $mac!(react_native_gradle);
    }
}

// commands we want to run the update nagger on
const UPDATE_NAGGER_CMDS: &'static [&'static str] = &[
    "releases",
    "issues",
    "repos",
    "projects",
    "info",
    "login",
    "difutil",
];

// it would be great if this could be a macro expansion as well
// but rust bug #37663 breaks location information then.
pub mod upload_dsym;
pub mod upload_proguard;
pub mod releases;
pub mod issues;
pub mod repos;
pub mod projects;
pub mod update;
pub mod uninstall;
pub mod info;
pub mod login;
pub mod send_event;
pub mod bash_hook;

pub mod react_native;
#[cfg(target_os="macos")]
pub mod react_native_xcode;
pub mod react_native_gradle;
pub mod react_native_codepush;

pub mod difutil;
pub mod difutil_find;
pub mod difutil_check;
pub mod difutil_uuid;

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
        .help_message("Print this help message.")
        .version(VERSION)
        .version_message("Print version information.")
        .about(ABOUT)
        .max_term_width(100)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("url")
            .value_name("URL")
            .long("url")
            .help("Fully qualified URL to the Sentry server.{n}[defaults to https://sentry.io/]"))
        .arg(Arg::with_name("auth_token")
            .value_name("AUTH_TOKEN")
            .long("auth-token")
            .help("Use the given Sentry auth token."))
        .arg(Arg::with_name("api_key")
            .value_name("API_KEY")
            .long("api-key")
            .help("The the given Sentry API key."))
        .arg(Arg::with_name("log_level")
            .value_name("LOG_LEVEL")
            .long("log-level")
            .help("Set the log output verbosity.{n}\
                   [valid levels: TRACE, DEBUG, INFO, WARN, ERROR]"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            let mut cmd = $name::make_app(
                App::new(stringify!($name).replace("_", "-").as_str()));

            // for legacy reasons
            if stringify!($name).starts_with("react_native_") {
                cmd = cmd.setting(AppSettings::Hidden);
            }

            app = app.subcommand(cmd);
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
    }).ok();

    config.configure_environment();

    macro_rules! execute_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            if let Some(sub_matches) = matches.subcommand_matches(&cmd) {
                let rv = $name::execute(&sub_matches, &config)?;
                if UPDATE_NAGGER_CMDS.iter().any(|x| x == &cmd) {
                    run_sentrycli_update_nagger(&config);
                }
                return Ok(rv);
            }
        }}
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}

fn run() -> Result<()> {
    prepare_environment();
    let mut cfg = Config::from_cli_config()?;
    match execute(env::args().collect(), &mut cfg) {
        Ok(()) => Ok(()),
        Err(err) => {
            // if the user hit an error, it might be time to run the update
            // nagger because maybe they tried to do something only newer
            // versions support.
            run_sentrycli_update_nagger(&cfg);
            Err(err)
        }
    }
}

/// Executes the command line application and exists the process.
pub fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(err) => {
            if let &ErrorKind::QuietExit(code) = err.kind() {
                process::exit(code);
            } else {
                print_error(&err);
                process::exit(1);
            }
        }
    }
}
