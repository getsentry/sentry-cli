use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List all events in your organization.")
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let api = Api::current();

    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;
    let events = api.list_organization_project_events(&org, &project)?;

    let mut table = Table::new();
    let row = table.title_row().add("Event ID").add("Date").add("Title");
    if matches.is_present("user") {
        row.add("User");
    }

    if matches.is_present("tags") {
        row.add("Tags");
    }

    for event in &events {
        let row = table.add_row();
        row.add(&event.event_id)
            .add(&event.date_created)
            .add(&event.title);

        if matches.is_present("user") {
            row.add(&event.user);
        }

        if matches.is_present("tags") {
            row.add(&event.tags);
        }
    }

    if table.is_empty() {
        println!("No events found");
    } else {
        table.print();
    }

    Ok(())
}
