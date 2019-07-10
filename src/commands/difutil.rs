use clap::{App, AppSettings, ArgMatches};
use failure::Error;

use crate::commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(difutil_bundle_sources);
        $mac!(difutil_find);
        $mac!(difutil_check);
        $mac!(difutil_id);
    };
}

pub fn make_app<'a, 'b: 'a>(mut app: App<'a, 'b>) -> App<'a, 'b> {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand(commands::$name::make_app(App::new(
                stringify!($name)[8..].replace('_', "-"),
            )));
        }};
    }

    app = app
        .about("Locate or analyze debug information files.")
        .setting(AppSettings::SubcommandRequiredElseHelp);
    each_subcommand!(add_subcommand);
    app
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) =
                matches.subcommand_matches(&stringify!($name)[8..].replace('_', "-"))
            {
                return Ok(commands::$name::execute(&sub_matches)?);
            }
        }};
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
