use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(difutil_bundle_sources);
        $mac!(difutil_find);
        $mac!(difutil_check);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(commands::$name::make_command(Command::new(
                stringify!($name)[8..].replace('_', "-"),
            )));
        }};
    }

    command = command
        .about("Locate or analyze debug information files.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name)[8..].replace('_', "-"))
            {
                return commands::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
