use anyhow::{bail, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::api::Api;

pub fn make_command(command: Command) -> Command {
    command
        .about("Fetch the latest event for an issue.")
        .arg(
            Arg::new("issue_id")
                .value_name("ISSUE_ID")
                .required(true)
                .help("The ID of the issue."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output as JSON."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    #[expect(clippy::unwrap_used, reason = "required argument")]
    let issue_id = matches.get_one::<String>("issue_id").unwrap();

    let event = match authenticated_api.get_issue_latest_event(issue_id)? {
        Some(v) => v,
        None => bail!("No event found for issue '{issue_id}'"),
    };

    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&event)?);
        return Ok(());
    }

    println!("Event ID:  {}", event.event_id);
    println!("Date:      {}", event.date_created);
    println!("Title:     {}", event.title);

    // print exception entry if present
    if let Some(entries) = &event.entries {
        for entry in entries {
            if entry.entry_type == "exception" {
                if let Some(values) = entry.data.get("values").and_then(|v| v.as_array()) {
                    for exc in values {
                        let exc_type = exc.get("type").and_then(|v| v.as_str()).unwrap_or_default();
                        let exc_value = exc
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        println!("\nException: {exc_type}: {exc_value}");

                        if let Some(frames) = exc
                            .get("stacktrace")
                            .and_then(|s| s.get("frames"))
                            .and_then(|f| f.as_array())
                        {
                            println!("\nStacktrace:");
                            for frame in frames {
                                let filename = frame
                                    .get("filename")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();
                                let lineno = frame.get("lineno").and_then(|v| v.as_u64());
                                let function = frame
                                    .get("function")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();
                                if let Some(n) = lineno {
                                    println!("  {filename}:{n} in {function}");
                                } else {
                                    println!("  {filename} in {function}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(tags) = &event.tags {
        if !tags.is_empty() {
            println!("\nTags:");
            for tag in tags {
                println!("  {}={}", tag.key, tag.value);
            }
        }
    }

    Ok(())
}
