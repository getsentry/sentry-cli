use anyhow::Result;
use clap::{ArgMatches, Command};
use log::info;

use crate::api::{Api, IssueChanges, IssueFilter};
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command.about("Bulk mute all selected issues.")
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let filter = IssueFilter::get_filter_from_matches(matches)?;
    let mut changes: IssueChanges = Default::default();

    info!(
        "Issuing a command for Organization: {} Project: {}",
        org, project
    );

    changes.new_status = Some("muted".into());

    if Api::current().bulk_update_issue(&org, &project, &filter, &changes)? {
        println!("Updated matching issues.");
        if let Some(status) = changes.new_status.as_ref() {
            println!("  new status: {status}");
        }
    } else {
        println!("No changes requested.");
    }
    Ok(())
}
