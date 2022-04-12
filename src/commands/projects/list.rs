use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List all projects for an organization.")
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let org = config.get_org(matches)?;
    let mut projects = api.list_organization_projects(&org)?;
    projects.sort_by_key(|p| {
        (
            p.team.as_ref().map_or(String::new(), |t| t.name.clone()),
            p.name.clone(),
        )
    });

    let mut table = Table::new();
    table
        .title_row()
        .add("ID")
        .add("Slug")
        .add("Team")
        .add("Name");

    for project in &projects {
        let team_name = &project
            .team
            .as_ref()
            .map_or(String::from("-"), |t| t.name.clone());

        table
            .add_row()
            .add(&project.id)
            .add(&project.slug)
            .add(team_name)
            .add(&project.name);
    }

    table.print();

    Ok(())
}
