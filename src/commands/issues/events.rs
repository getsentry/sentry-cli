use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command
        .about("List events for a specific issue.")
        .arg(
            Arg::new("issue_id")
                .required(true)
                .value_name("ISSUE_ID")
                .help("The issue ID (e.g., PROJ-123 or full UUID)."),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .short('l')
                .value_name("LIMIT")
                .default_value("50")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of events to return."),
        )
        .arg(
            Arg::new("sort")
                .long("sort")
                .value_name("SORT")
                .default_value("-timestamp")
                .help("Sort field (e.g., -timestamp, timestamp)."),
        )
        .arg(
            Arg::new("period")
                .long("period")
                .value_name("PERIOD")
                .default_value("14d")
                .help("Time period (e.g., 24h, 7d, 14d)."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let issue_id = matches
        .get_one::<String>("issue_id")
        .expect("issue_id is required");
    let limit = *matches
        .get_one::<usize>("limit")
        .expect("limit has default value");
    let sort = matches.get_one::<String>("sort").map(|s| s.as_str());
    let period = matches.get_one::<String>("period").map(|s| s.as_str());

    let api = Api::current();
    let events =
        api.authenticated()?
            .list_issue_events(&org, issue_id, Some(limit), sort, period)?;

    if events.is_empty() {
        println!("No events found for issue {issue_id}");
        return Ok(());
    }

    let event_count = events.len();
    println!("Events for {issue_id} (showing {event_count}):");
    println!();

    let mut table = Table::new();
    table
        .title_row()
        .add("Event ID")
        .add("Timestamp")
        .add("Environment")
        .add("Release");

    for event in &events {
        let env = event
            .tags
            .as_ref()
            .and_then(|tags| tags.iter().find(|t| t.key == "environment"))
            .map(|t| t.value.as_str())
            .unwrap_or("-");
        let release = event
            .tags
            .as_ref()
            .and_then(|tags| tags.iter().find(|t| t.key == "release"))
            .map(|t| t.value.as_str())
            .unwrap_or("-");
        let timestamp = event.date_created.as_deref().unwrap_or("-");

        table
            .add_row()
            .add(&event.event_id)
            .add(timestamp)
            .add(env)
            .add(release);
    }

    table.print();

    Ok(())
}
