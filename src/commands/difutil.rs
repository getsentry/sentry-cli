use clap::{ArgMatches, Command};
use failure::Error;

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(difutil_bundle_sources);
        $mac!(difutil_find);
        $mac!(difutil_check);
    };
}

pub fn make_app(mut app: Command) -> Command {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand(commands::$name::make_app(Command::new(
                stringify!($name)[8..].replace('_', "-"),
            )));
        }};
    }

    app = app
        .about("Locate or analyze debug information files.")
        .subcommand_required(true)
        .arg_required_else_help(true);
    each_subcommand!(add_subcommand);
    app
}

pub fn execute(matches: &ArgMatches) -> Result<(), Error> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name)[8..].replace('_', "-"))
            {
                return commands::$name::execute(&sub_matches);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
