use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod list;
pub mod run;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(list);
        $mac!(run);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::monitors::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage monitors on Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Legacy command, left hidden for backward compatibility
        .hide(true);

    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::monitors::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
