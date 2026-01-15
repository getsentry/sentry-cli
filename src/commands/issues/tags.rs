use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command
        .about("Get tag value distribution for an issue.")
        .arg(
            Arg::new("issue_id")
                .required(true)
                .value_name("ISSUE_ID")
                .help("The issue ID (e.g., PROJ-123 or full UUID)."),
        )
        .arg(
            Arg::new("key")
                .long("key")
                .short('k')
                .required(true)
                .value_name("TAG_KEY")
                .help("The tag key to get values for (e.g., browser, os, environment)."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let issue_id = matches
        .get_one::<String>("issue_id")
        .expect("issue_id is required");
    let tag_key = matches.get_one::<String>("key").expect("key is required");

    let api = Api::current();
    let tag_values = api
        .authenticated()?
        .get_issue_tag_values(&org, issue_id, tag_key)?;

    println!(
        "Tag: {} ({} unique values)",
        tag_values.name, tag_values.total_values
    );
    println!();

    if tag_values.top_values.is_empty() {
        println!("No values found for tag '{tag_key}'");
        return Ok(());
    }

    // Calculate total for percentage
    let total: u64 = tag_values.top_values.iter().map(|v| v.count).sum();

    for value in &tag_values.top_values {
        let display_value = value.value.as_deref().unwrap_or("(none)");
        let percentage = if total > 0 {
            (value.count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let count = value.count;
        println!("  {display_value:30} {count:>8} events ({percentage:.0}%)");
    }

    Ok(())
}
