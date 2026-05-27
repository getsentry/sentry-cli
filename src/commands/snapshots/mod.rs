use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod diff;
pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(diff);
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::snapshots::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("[EXPERIMENTAL] Manage and compare snapshots.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::snapshots::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
