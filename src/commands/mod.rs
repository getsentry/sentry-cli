//! This module implements the root command of the CLI tool.

use std::env;
use std::fmt;
use std::process;

use clap::{App, AppSettings, Arg, ArgMatches};
use failure::{bail, Error};
use log::{debug, info};

use crate::api::Api;
use crate::config::{prepare_environment, Auth, Config};
use crate::constants::{ARCH, PLATFORM, VERSION};
use crate::utils::system::{print_error, QuietExit};
use crate::utils::update::run_sentrycli_update_nagger;

const ABOUT: &str = "
Command line utility for Sentry.

This tool helps you manage remote resources on a Sentry server like
sourcemaps, debug symbols or releases.  Use `--help` on the subcommands
to learn more about them.";

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dif);
        $mac!(upload_dsym);
        $mac!(upload_proguard);
        $mac!(releases);
        $mac!(issues);
        $mac!(repos);
        $mac!(projects);
        $mac!(monitors);
        #[cfg(not(feature = "managed"))]
        $mac!(update);
        #[cfg(not(feature = "managed"))]
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
        #[cfg(target_os = "macos")]
        $mac!(react_native_xcode);
        $mac!(react_native_gradle);
    };
}

// commands we want to run the update nagger on
const UPDATE_NAGGER_CMDS: &[&str] = &[
    "releases", "issues", "repos", "projects", "monitors", "info", "login", "difutil",
];

// it would be great if this could be a macro expansion as well
// but rust bug #37663 breaks location information then.
pub mod bash_hook;
pub mod info;
pub mod issues;
pub mod login;
pub mod monitors;
pub mod projects;
pub mod releases;
pub mod repos;
pub mod send_event;
pub mod uninstall;
pub mod update;
pub mod upload_dif;
pub mod upload_dsym;
pub mod upload_proguard;

pub mod react_native;
pub mod react_native_appcenter;
pub mod react_native_codepush;
pub mod react_native_gradle;
#[cfg(target_os = "macos")]
pub mod react_native_xcode;

pub mod difutil;
pub mod difutil_bundle_sources;
pub mod difutil_check;
pub mod difutil_find;
pub mod difutil_id;

fn preexecute_hooks() -> Result<bool, Error> {
    return sentry_react_native_xcode_wrap();

    #[cfg(target_os = "macos")]
    fn sentry_react_native_xcode_wrap() -> Result<bool, Error> {
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
    fn sentry_react_native_xcode_wrap() -> Result<bool, Error> {
        Ok(false)
    }
}

fn configure_args(config: &mut Config, matches: &ArgMatches<'_>) -> Result<(), Error> {
    if let Some(url) = matches.value_of("url") {
        config.set_base_url(url);
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

fn add_commands<'a, 'b>(mut app: App<'a, 'b>) -> App<'a, 'b> {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            let mut cmd = $name::make_app(App::new(stringify!($name).replace("_", "-").as_str()));

            // for legacy reasons
            if stringify!($name).starts_with("react_native_") {
                cmd = cmd.setting(AppSettings::Hidden);
            }

            app = app.subcommand(cmd);
        }};
    }

    each_subcommand!(add_subcommand);
    app
}

#[allow(clippy::cognitive_complexity)]
fn run_command(matches: &ArgMatches<'_>) -> Result<(), Error> {
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

struct DebugArgs<'a>(Vec<&'a str>);

impl<'a> fmt::Display for DebugArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, arg) in self.0.iter().enumerate() {
            if idx > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:?}", arg)?;
        }
        Ok(())
    }
}

/// Given an argument vector and a `Config` this executes the
/// command line and returns the result.
pub fn execute(args: &[String]) -> Result<(), Error> {
    let mut config = Config::from_cli_config()?;

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
        .arg(Arg::with_name("url").value_name("URL").long("url").help(
            "Fully qualified URL to the Sentry server.{n}\
             [defaults to https://sentry.io/]",
        ))
        .arg(
            Arg::with_name("auth_token")
                .value_name("AUTH_TOKEN")
                .long("auth-token")
                .help("Use the given Sentry auth token."),
        )
        .arg(
            Arg::with_name("api_key")
                .value_name("API_KEY")
                .long("api-key")
                .help("The given Sentry API key."),
        )
        .arg(
            Arg::with_name("log_level")
                .value_name("LOG_LEVEL")
                .long("log-level")
                .global(true)
                .help(
                    "Set the log output verbosity.{n}\
                     [valid levels: TRACE, DEBUG, INFO, WARN, ERROR]",
                ),
        );

    app = add_commands(app);
    let matches = app.get_matches_from_safe(&args[..])?;
    configure_args(&mut config, &matches)?;

    // bind the config to the process and fetch an immutable reference to it
    config.bind_to_process();
    info!(
        "Loaded config from {}",
        Config::current().get_filename().display()
    );

    debug!(
        "sentry-cli version: {}, platform: \"{}\", architecture: \"{}\"",
        VERSION, PLATFORM, ARCH
    );

    info!(
        "sentry-cli was invoked with the following command line: {}",
        DebugArgs(args.iter().map(String::as_str).collect())
    );

    run_command(&matches)
}

fn run() -> Result<(), Error> {
    prepare_environment();
    match execute(&env::args().collect::<Vec<String>>()) {
        Ok(()) => Ok(()),
        Err(err) => {
            // if the user hit an error, it might be time to run the update
            // nagger because maybe they tried to do something only newer
            // versions support.
            debug!("error: running update nagger");
            run_sentrycli_update_nagger();
            Err(err)
        }
    }
}

fn setup() {
    use crate::utils::logging::Logger;

    crate::utils::system::init_backtrace();
    crate::utils::system::load_dotenv();

    // we use debug internally but our log handler then rejects to a lower limit.
    // This is okay for our uses but not as efficient.
    log::set_max_level(log::LevelFilter::Debug);

    // if we work with crash reporting we initialize the sentry system.  This
    // also configures the logger.
    #[cfg(feature = "with_crash_reporting")]
    {
        crate::utils::crashreporting::setup(Box::new(Logger));
    }
    #[cfg(not(feature = "with_crash_reporting"))]
    {
        static LOGGER: Logger = Logger;
        log::set_logger(&Logger);
    }
}

/// Executes the command line application and exits the process.
pub fn main() {
    setup();
    let result = run();

    let status_code = match result {
        Ok(()) => 0,
        Err(err) => {
            if let Some(&QuietExit(code)) = err.downcast_ref() {
                code
            } else {
                print_error(&err);
                #[cfg(feature = "with_crash_reporting")]
                {
                    crate::utils::crashreporting::try_report_to_sentry(&err);
                }
                1
            }
        }
    };

    // before we shut down we unbind the api to give the connection pool
    // a chance to collect.  Not doing so has shown to cause hung threads
    // on windows.
    Api::dispose_pool();
    process::exit(status_code);
}
