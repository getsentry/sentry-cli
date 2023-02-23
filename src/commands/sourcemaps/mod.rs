use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt;

pub mod explain;
pub mod inject;
pub mod resolve;
pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(explain);
        $mac!(inject);
        $mac!(resolve);
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::sourcemaps::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage sourcemaps for Sentry releases.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        .release_arg();
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::sourcemaps::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
