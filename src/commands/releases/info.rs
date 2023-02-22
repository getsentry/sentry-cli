use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;
use crate::utils::logging::is_quiet_mode;
use crate::utils::system::QuietExit;

pub fn make_command(command: Command) -> Command {
    command
        .about("Print information about a release.")
        .allow_hyphen_values(true)
        .version_arg()
        .arg(
            Arg::new("show_projects")
                .short('P')
                .long("show-projects")
                .help("Display the Projects column"),
        )
        .arg(
            Arg::new("show_commits")
                .short('C')
                .long("show-commits")
                .help("Display the Commits column"),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let release = api.get_release(&org, project.as_deref(), version)?;

    if is_quiet_mode() {
        if release.is_none() {
            return Err(QuietExit(1).into());
        }
        return Ok(());
    }

    if let Some(release) = release {
        let mut tbl = Table::new();
        let title_row = tbl.title_row().add("Version").add("Date created");

        if release.last_event.is_some() {
            title_row.add("Last event");
        }

        if matches.contains_id("show_projects") {
            title_row.add("Projects");
        }

        if matches.contains_id("show_commits") {
            title_row.add("Commits");
        }

        let data_row = tbl
            .add_row()
            .add(&release.version)
            .add(release.date_created);

        if let Some(last_event) = release.last_event {
            data_row.add(last_event);
        }

        if matches.contains_id("show_projects") {
            let project_slugs = release
                .projects
                .into_iter()
                .map(|p| p.slug)
                .collect::<Vec<_>>();
            if !project_slugs.is_empty() {
                data_row.add(project_slugs.join("\n"));
            } else {
                data_row.add("-");
            }
        }

        if matches.contains_id("show_commits") {
            if let Ok(Some(commits)) = api.get_release_commits(&org, project.as_deref(), version) {
                if !commits.is_empty() {
                    data_row.add(
                        commits
                            .into_iter()
                            .map(|c| c.id)
                            .collect::<Vec<String>>()
                            .join("\n"),
                    );
                } else {
                    data_row.add("-");
                }
            } else {
                data_row.add("-");
            }
        }

        tbl.print();
    } else {
        return Err(QuietExit(1).into());
    }
    Ok(())
}
