use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::utils::args::ArgExt;

pub mod mute;
pub mod resolve;
pub mod unresolve;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(mute);
        $mac!(resolve);
        $mac!(unresolve);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::issues::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage issues in Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("status")
                .long("status")
                .short('s')
                .value_name("STATUS")
                .global(true)
                .value_parser(["resolved", "muted", "unresolved"])
                .help("Select all issues matching a given status."),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .short('a')
                .global(true)
                .help("Select all issues (this might be limited)."),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .short('i')
                .value_name("ID")
                .action(ArgAction::Append)
                .global(true)
                .help("Select the issue with the given ID."),
        );
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::issues::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
