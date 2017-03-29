use clap::{App, ArgMatches, AppSettings};

use prelude::*;
use config::Config;
use utils::ArgExt;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads react-native projects from within a gradle build step")
        .setting(AppSettings::Hidden)
        .org_project_args()
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    Err(Error::from("this command is currently not implemented"))
}
