use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command
        .about("List all events in your organization.")
        .arg(
            Arg::new("show_user")
                .long("show-user")
                .short('U')
                .help("Display the Users column."),
        )
        .arg(
            Arg::new("show_tags")
                .long("show-tags")
                .short('T')
                .help("Display the Tags column."),
        )
        .arg(
            Arg::new("max_rows")
                .long("max-rows")
                .value_name("MAX_ROWS")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of rows to print."),
        )
        .arg(
            Arg::new("pages")
                .long("pages")
                .value_name("PAGES")
                .default_value("5")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of pages to fetch (100 events/page)."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;
    let pages = *matches.get_one("pages").unwrap();
    let api = Api::current();

    let events = api.list_organization_project_events(&org, &project, pages)?;

    let mut table = Table::new();
    let title_row = table.title_row().add("Event ID").add("Date").add("Title");

    if matches.contains_id("show_user") {
        title_row.add("User");
    }

    if matches.contains_id("show_tags") {
        title_row.add("Tags");
    }

    let max_rows = std::cmp::min(
        events.len(),
        *matches.get_one("max_rows").unwrap_or(&std::usize::MAX),
    );

    if let Some(events) = events.get(..max_rows) {
        for event in events {
            let row = table.add_row();
            row.add(event.event_id)
                .add(&event.date_created)
                .add(&event.title);

            if matches.contains_id("show_user") {
                if let Some(user) = &event.user {
                    row.add(user);
                } else {
                    row.add("-");
                }
            }

            if matches.contains_id("show_tags") {
                if let Some(tags) = &event.tags {
                    row.add(
                        tags.iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                } else {
                    row.add("-");
                }
            }
        }
    }

    if table.is_empty() {
        println!("No events found");
    } else {
        table.print();
    }

    Ok(())
}
