use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt;

pub mod archive;
pub mod delete;
pub mod finalize;
pub mod info;
pub mod list;
pub mod new;
pub mod propose_version;
pub mod restore;
pub mod set_commits;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(archive);
        $mac!(delete);
        $mac!(finalize);
        $mac!(info);
        $mac!(list);
        $mac!(new);
        $mac!(propose_version);
        $mac!(restore);
        $mac!(set_commits);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::releases::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage releases on Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        // Backward compatibility with `releases files <VERSION>` commands.
        .subcommand(
            crate::commands::files::make_command(Command::new("files"))
                .allow_hyphen_values(true)
                .version_arg()
                .hide(true),
        )
        // Backward compatibility with `releases deploys <VERSION>` commands.
        .subcommand(
            crate::commands::deploys::make_command(Command::new("deploys"))
                .allow_hyphen_values(true)
                .version_arg()
                .hide(true),
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
                return crate::commands::releases::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);

    // To preserve backward compatibility
    if let Some(sub_matches) = matches.subcommand_matches("files") {
        return crate::commands::files::execute(sub_matches);
    }
    // To preserve backward compatibility
    if let Some(sub_matches) = matches.subcommand_matches("deploys") {
        return crate::commands::deploys::execute(sub_matches);
    }

    unreachable!();
}
