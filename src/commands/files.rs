use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(files_delete);
        $mac!(files_list);
        $mac!(files_upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::$name::make_command(Command::new(
                stringify!($name)[6..].replace('_', "-"),
            )));
        }};
    }

    command = command
        .about("Manage release artifacts.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        .release_arg()
        .subcommand(
            crate::commands::sourcemaps_upload::make_command(Command::new("upload-sourcemaps"))
                .hide(true),
        );

    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name)[6..].replace('_', "-"))
            {
                return crate::commands::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);

    // To preserve backward compatibility
    if let Some(sub_matches) = matches.subcommand_matches("upload-sourcemaps") {
        return crate::commands::sourcemaps_upload::execute(sub_matches);
    }

    unreachable!();
}
