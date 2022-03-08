use clap::{ArgMatches, Command};
use failure::Error;

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(react_native_gradle);
        $mac!(react_native_appcenter);
        #[cfg(target_os = "macos")]
        $mac!(react_native_xcode);
    };
}

pub fn make_app(mut app: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand(commands::$name::make_app(Command::new(
                &stringify!($name)[13..],
            )));
        }};
    }

    app = app
        .about("Upload build artifacts for react-native projects.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    app
}

pub fn execute(matches: &ArgMatches) -> Result<(), Error> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) = matches.subcommand_matches(&stringify!($name)[13..]) {
                return commands::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
