//! Implements a command for managing repos.
use clap::{App, AppSettings, ArgMatches};
use failure::Error;

use api::Api;
use config::Config;
use utils::args::validate_org;
use utils::formatting::Table;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Manage repositories on Sentry.")
        (setting: AppSettings::SubcommandRequiredElseHelp)
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@subcommand list =>
            (about: "List all repositories in your organization.")
        )
    )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let api = Api::get_current();

    let config = Config::get_current();
    let org = config.get_org(matches)?;
    let repos = api.list_organization_repos(&org)?;

    let mut table = Table::new();
    table.title_row().add("Name").add("Provider").add("URL");

    for repo in &repos {
        table
            .add_row()
            .add(&repo.name)
            .add(&repo.provider.name)
            .add(&repo.url.as_ref().map(|x| x.as_str()).unwrap_or("-"));
    }

    if table.is_empty() {
        println!("No repos found");
    } else {
        table.print();
    }

    Ok(())
}
