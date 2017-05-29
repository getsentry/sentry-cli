use clap::{App, ArgMatches};

use prelude::*;
use config::Config;

use commands::{react_native_xcode, react_native_gradle, react_native_codepush};


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("provides helpers for react-native.")
        .subcommand(react_native_xcode::make_app(App::new("xcode")))
        .subcommand(react_native_gradle::make_app(App::new("gradle")))
        .subcommand(react_native_codepush::make_app(App::new("codepush")))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("xcode") {
        react_native_xcode::execute(&sub_matches, config)
    } else if let Some(sub_matches) = matches.subcommand_matches("gradle") {
        react_native_gradle::execute(&sub_matches, config)
    } else if let Some(sub_matches) = matches.subcommand_matches("codepush") {
        react_native_codepush::execute(&sub_matches, config)
    } else {
        unreachable!();
    }
}
