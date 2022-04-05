use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(debug_files_bundle_sources);
        $mac!(debug_files_find);
        $mac!(debug_files_check);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(commands::$name::make_command(Command::new(
                stringify!($name)[12..].replace('_', "-"),
            )));
        }};
    }

    command = command
        .about("Locate, analyze or upload debug information files.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .visible_alias("difutil");
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name)[12..].replace('_', "-"))
            {
                return commands::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
