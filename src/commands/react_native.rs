use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(react_native_gradle);
        $mac!(react_native_appcenter);
        #[cfg(target_os = "macos")]
        $mac!(react_native_xcode);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(commands::$name::make_command(Command::new(
                &stringify!($name)[13..],
            )));
        }};
    }

    command = command
        .about("Upload build artifacts for react-native projects.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
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
