use anyhow::Result;
use clap::{ArgMatches, Command};

// Keep consistent import patterns with other grouped commands; currently no ArgExt usage here

pub mod upload;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload);
    };
}

pub fn make_command(mut command: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            command = command.subcommand(crate::commands::dart_symbol_map::$name::make_command(
                Command::new(stringify!($name).replace('_', "-")),
            ));
        }};
    }

    command = command
        .about("Manage Dart/Flutter symbol maps for Sentry.")
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
                return crate::commands::dart_symbol_map::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}


