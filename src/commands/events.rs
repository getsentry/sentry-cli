//! Implements a command for issue management.
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Error;
use log::info;

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Manage events on Sentry.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_project_args()
        .arg(
            Arg::with_name("user")
                .long("user")
                .short("u")
                .help("Include user's info into the list."),
        )
        .arg(
            Arg::with_name("tags")
                .long("tags")
                .short("t")
                .help("Include tags into the list."),
        )
        .subcommand(App::new("list").about("List all events for a project."))
}

pub fn execute(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let config = Config::current();
    let api = Api::current();
    let (org, project) = config.get_org_and_project(matches)?;

    info!("Get events for Organization: {} Project: {}", org, project);

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

    table.print();

    Ok(())
}
