use anyhow::Result;
use clap::{ArgMatches, Command};
use log::debug;

use crate::api::{Api, Organization};
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List all organizations available to the authenticated token.")
}

pub fn execute(_matches: &ArgMatches) -> Result<()> {
    let api = Api::current();

    // Query regions available to the current CLI user
    let regions = api.list_available_regions()?;

    let mut organizations: Vec<Organization> = vec![];
    debug!("Available regions: {:?}", regions);

    // Self-hosted instances won't have a region instance or prefix, so we
    // need to check before fanning out.
    if regions.len() > 1 {
        for region in regions {
            organizations.append(&mut api.list_organizations(Some(&region))?)
        }
    } else {
        organizations.append(&mut api.list_organizations(None)?)
    }

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
