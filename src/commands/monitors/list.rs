use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command
        .about("List all monitors for an organization.")
        .org_arg()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let org = config.get_org(matches)?;
    let mut monitors = api.list_organization_monitors(&org)?;
    monitors.sort_by_key(|p| (p.name.clone()));

    let mut table = Table::new();
    table.title_row().add("ID").add("Name").add("Status");

    for monitor in &monitors {
        table
            .add_row()
            .add(&monitor.id)
            .add(&monitor.name)
            .add(&monitor.status);
    }

    table.print();

    Ok(())
}
