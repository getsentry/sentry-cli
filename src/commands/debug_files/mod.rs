use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod bundle_sources;
pub mod check;
pub mod find;
pub mod print_sources;
pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(bundle_sources);
        $mac!(check);
        $mac!(find);
        $mac!(print_sources);
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::debug_files::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Locate, analyze or upload debug information files.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .visible_alias("dif")
        .alias("difutil");
    each_subcommand!(add_subcommand);
    command
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name).replace('_', "-"))
            {
                return crate::commands::debug_files::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
