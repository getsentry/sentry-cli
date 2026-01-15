use std::fs::File;
use std::io::Write as _;
use std::path::Path;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command
        .about("List or download attachments for an event.")
        .arg(
            Arg::new("event_id")
                .required(true)
                .value_name("EVENT_ID")
                .help("The event ID."),
        )
        .arg(
            Arg::new("attachment_id")
                .value_name("ATTACHMENT_ID")
                .help("The attachment ID to download (optional, lists all if omitted)."),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("PATH")
                .help("Output file path for download (required when downloading)."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;
    let event_id = matches
        .get_one::<String>("event_id")
        .expect("event_id is required");
    let attachment_id = matches.get_one::<String>("attachment_id");
    let output_path = matches.get_one::<String>("output");

    let api = Api::current();
    let authenticated = api.authenticated()?;

    match attachment_id {
        None => {
            // List mode
            let attachments = authenticated.list_event_attachments(&org, &project, event_id)?;

            if attachments.is_empty() {
                println!("No attachments found for event {event_id}");
                return Ok(());
            }

            println!("Attachments for event {event_id}:");
            println!();

            let mut table = Table::new();
            table
                .title_row()
                .add("ID")
                .add("Name")
                .add("Type")
                .add("Size");

            for att in &attachments {
                let size = format_size(att.size);
                table
                    .add_row()
                    .add(&att.id)
                    .add(&att.name)
                    .add(att.mimetype.as_deref().unwrap_or(&att.attachment_type))
                    .add(&size);
            }

            table.print();
        }
        Some(att_id) => {
            // Download mode
            let output = match output_path {
                Some(p) => p.clone(),
                None => bail!("--output is required when downloading an attachment"),
            };

            let data = authenticated.download_event_attachment(&org, &project, event_id, att_id)?;

            let path = Path::new(&output);
            let mut file = File::create(path)?;
            file.write_all(&data)?;

            let size = format_size(data.len() as u64);
            println!("Downloaded: {output} ({size})");
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
