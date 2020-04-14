//! Implements a command for managing projects.
use clap::{App, AppSettings, ArgMatches};
use failure::Error;

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Manage projects on Sentry.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_arg()
        .subcommand(App::new("list").about("List all projects for an organization."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::current();
    let api = Api::current();
    let org = config.get_org(matches)?;
    let mut projects = api.list_organization_projects(&org)?;
    projects.sort_by_key(|p| (p.team.name.clone(), p.name.clone()));

    let mut table = Table::new();
    table
        .title_row()
        .add("ID")
        .add("Slug")
        .add("Team")
        .add("Name");

    for project in &projects {
        table
            .add_row()
            .add(&project.id)
            .add(&project.slug)
            .add(&project.team.name)
            .add(&project.name);
    }

    table.print();

    Ok(())
}
