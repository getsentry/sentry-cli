use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::utils::args::ArgExt;

pub mod list;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(list);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::events::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage events on Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        .arg(
            Arg::with_name("user")
                .long("user")
                .short('u')
                .global(true)
                .help("Include user's info into the list."),
        )
        .arg(
            Arg::with_name("tags")
                .long("tags")
                .short('t')
                .global(true)
                .help("Include tags into the list."),
        )
        .arg(
            Arg::new("max-rows")
                .long("max-rows")
                .global(true)
                .help("Max of rows for a table."),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .global(true)
                .default_value("10")
                .help("Limit of requests."),
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
                return crate::commands::events::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
