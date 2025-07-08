#![cfg(feature = "unstable-mobile-app")]

use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt;

pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::mobile_app::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("[EXPERIMENTAL] Manage mobile apps.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        // TODO: Remove this when ready for release
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
                return crate::commands::mobile_app::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
