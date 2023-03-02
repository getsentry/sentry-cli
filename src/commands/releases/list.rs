use anyhow::Result;
use chrono::Utc;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::{HumanDuration, Table};

pub fn make_command(command: Command) -> Command {
    command
        .about("List the most recent releases.")
        .arg(
            Arg::new("show_projects")
                .short('P')
                .long("show-projects")
                .help("Display the Projects column"),
        )
        .arg(
            Arg::new("raw")
                .short('R')
                .long("raw")
                .help("Print raw, delimiter separated list of releases. [defaults to new line]"),
        )
        .arg(
            Arg::new("delimiter")
                .short('D')
                .long("delimiter")
                .takes_value(true)
                .requires("raw")
                .help("Delimiter for the --raw flag"),
        )
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(Arg::new("no_abbrev").long("no-abbrev").hide(true))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let project = config.get_project(matches).ok();
    let releases = api.list_releases(&config.get_org(matches)?, project.as_deref())?;

    if matches.contains_id("raw") {
        let versions = releases
            .iter()
            .map(|release_info| release_info.version.clone())
            .collect::<Vec<_>>()
            .join(
                matches
                    .get_one::<String>("delimiter")
                    .map(String::as_str)
                    .unwrap_or("\n"),
            );

        println!("{versions}");
        return Ok(());
    }

    let mut table = Table::new();
    let title_row = table.title_row();
    title_row.add("Released").add("Version");
    if matches.contains_id("show_projects") {
        title_row.add("Projects");
    }
    title_row.add("New Events").add("Last Event");
    for release_info in releases {
        let row = table.add_row();
        if let Some(date) = release_info.date_released {
            row.add(format!(
                "{} ago",
                HumanDuration(Utc::now().signed_duration_since(date))
            ));
        } else {
            row.add("(unreleased)");
        }
        row.add(&release_info.version);
        if matches.contains_id("show_projects") {
            let project_slugs = release_info
                .projects
                .into_iter()
                .map(|p| p.slug)
                .collect::<Vec<_>>();
            if !project_slugs.is_empty() {
                row.add(project_slugs.join("\n"));
            } else {
                row.add("-");
            }
        }
        row.add(release_info.new_groups);
        if let Some(date) = release_info.last_event {
            row.add(format!(
                "{} ago",
                HumanDuration(Utc::now().signed_duration_since(date))
            ));
        } else {
            row.add("-");
        }
    }
    table.print();
    Ok(())
}
