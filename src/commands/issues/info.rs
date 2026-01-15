use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command
        .about("Get detailed information about a specific issue.")
        .arg(
            Arg::new("issue_id")
                .required(true)
                .value_name("ISSUE_ID")
                .help("The issue ID (e.g., PROJ-123 or full UUID)."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let issue_id = matches.get_one::<String>("issue_id").unwrap();

    let api = Api::current();
    let authenticated = api.authenticated()?;

    let issue = authenticated.get_issue_details(&org, issue_id)?;
    let latest_event = authenticated
        .get_issue_latest_event(&org, issue_id)
        .ok()
        .flatten();

    println!("Issue: {}", issue.short_id);
    println!("Title: {}", issue.title);
    if let Some(culprit) = &issue.culprit {
        println!("Culprit: {}", culprit);
    }
    println!("Status: {}", issue.status);
    println!("Level: {}", issue.level);
    println!("Events: {}", issue.count);
    println!("Users: {}", issue.user_count);
    println!("First Seen: {}", issue.first_seen);
    println!("Last Seen: {}", issue.last_seen);
    println!("Link: {}", issue.permalink);

    if let Some(event) = latest_event {
        println!();
        println!("Latest Event: {}", event.event_id);
        if let Some(date) = &event.date_created {
            println!("  Timestamp: {}", date);
        }
        if let Some(tags) = &event.tags {
            for tag in tags {
                if tag.key == "environment" || tag.key == "release" {
                    println!("  {}: {}", tag.key, tag.value);
                }
            }
        }
    }

    Ok(())
}
