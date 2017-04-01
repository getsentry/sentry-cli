//! Implements a command for managing repos.
use crates::clap::{App, AppSettings, Arg, ArgMatches};

use prelude::*;
use config::Config;
use utils::{ArgExt, Table};
use api::Api;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("manage repos on Sentry")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_arg()
        .subcommand(App::new("list")
            .about("List all repos for an organization"))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let api = Api::new(config);

    let org = config.get_org(matches)?;
    let repos = api.list_organization_repos(&org)?;

    let mut table = Table::new();
    table.title_row()
        .add("Name")
        .add("Provider")
        .add("URL");

    for repo in &repos {
        table.add_row()
            .add(&repo.name)
            .add(&repo.provider.name)
            .add(&repo.url);
    }

    table.print();

    Ok(())
}
