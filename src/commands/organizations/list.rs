use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List all organizations available to the authenticated token.")
}

pub fn execute(_matches: &ArgMatches) -> Result<()> {
    let api = Api::current();
    let mut organizations = api.list_organizations()?;

    organizations.sort_by_key(|o| o.name.clone().to_lowercase());

    let mut table = Table::new();
    table
        .title_row()
        .add("ID")
        .add("Name")
        .add("Slug")
        .add("Date Created")
        .add("Early Adopter")
        .add("Requires 2FA");

    for organization in &organizations {
        table
            .add_row()
            .add(&organization.id)
            .add(&organization.name)
            .add(&organization.slug)
            .add(&organization.date_created.format("%F"))
            .add(organization.is_early_adopter)
            .add(organization.require_2fa);
    }

    table.print();

    Ok(())
}
