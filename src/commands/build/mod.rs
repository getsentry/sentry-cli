use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt as _;

pub mod download;
pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(download);
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::build::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage builds.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true);
    each_subcommand!(add_subcommand);
    command = command.subcommand(
        crate::commands::snapshots::upload::make_command(Command::new("snapshots"))
            .hide(true)
            .about("[DEPRECATED] Use `sentry-cli snapshots upload` instead."),
    );
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::build::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    if let Some(sub_matches) = matches.subcommand_matches("snapshots") {
        eprintln!(
            "WARNING: `sentry-cli build snapshots` is deprecated. \
             Use `sentry-cli snapshots upload` instead."
        );
        return crate::commands::snapshots::upload::execute(sub_matches);
    }
    unreachable!();
}
