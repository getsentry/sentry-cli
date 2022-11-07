use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List all repositories in your organization.")
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let api = Api::current();

    let config = Config::current();
    let org = config.get_org(matches)?;
    let repos = api.list_organization_repos(&org)?;

    let mut table = Table::new();
    table.title_row().add("Name").add("Provider").add("URL");

    for repo in &repos {
        table
            .add_row()
            .add(&repo.name)
            .add(&repo.provider.name)
            .add(repo.url.as_deref().unwrap_or("-"));
    }

    if table.is_empty() {
        println!("No repos found");
    } else {
        table.print();
    }

    Ok(())
}
