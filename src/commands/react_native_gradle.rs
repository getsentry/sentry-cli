use clap::{App, ArgMatches};

use prelude::*;
use config::Config;
use utils::ArgExt;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads react-native projects from within a gradle build step")
        .org_project_args()
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    Ok(())
}
