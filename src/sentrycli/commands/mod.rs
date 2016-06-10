use std::env;
use std::process;

use clap::{Arg, App, AppSettings};

use CliResult;
use utils::make_subcommand;
pub use config::{Config, Auth};

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dsym);
        $mac!(extract_iosds_symbols);
        $mac!(releases);
        $mac!(update);
        $mac!(uninstall);
        $mac!(info);
        $mac!(login);
    }
}

macro_rules! import_subcommand {
    ($name:ident) => { mod $name; }
}
each_subcommand!(import_subcommand);

pub fn execute(args: Vec<String>, config: &mut Config) -> CliResult<()> {
    let mut app = App::new("sentry-cli")
        .version(env!("CARGO_PKG_VERSION"))
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
             .help("The sentry API key to use"));

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
        config.auth = Auth::Key(api_key.to_owned());
    }
    if let Some(auth_token) = matches.value_of("auth_token") {
        config.auth = Auth::Token(auth_token.to_owned());
    }

    macro_rules! execute_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            if let Some(sub_matches) = matches.subcommand_matches(cmd) {
                return $name::execute(&sub_matches, &config);
            }
        }}
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}

pub fn run() -> CliResult<()> {
    execute(env::args().collect(), &mut Config::from_cli_config()?)
}

pub fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(ref err) => err.exit(),
    }
}
