use clap::{App, ArgMatches, AppSettings};

use prelude::*;

use commands;

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(react_native_gradle);
        $mac!(react_native_codepush);
        #[cfg(target_os="macos")]
        $mac!(react_native_xcode);
    }
}

pub fn make_app<'a, 'b: 'a>(mut app: App<'a, 'b>) -> App<'a, 'b> {
    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand(commands::$name::make_app(
                App::new(&stringify!($name)[13..])));
        }}
    }

    app = app
        .about("Upload build artifacts for react-native projects.")
        .setting(AppSettings::SubcommandRequiredElseHelp);
    each_subcommand!(add_subcommand);
    app
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    macro_rules! execute_subcommand {
        ($name:ident) => {{
            if let Some(sub_matches) = matches.subcommand_matches(&stringify!($name)[13..]) {
                return Ok(commands::$name::execute(&sub_matches)?);
            }
        }}
    }
    each_subcommand!(execute_subcommand);
    unreachable!();
}
