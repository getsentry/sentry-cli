use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::api::{Api, IssueChanges, IssueFilter};
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command.about("Bulk mute all selected issues.")
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let filter = IssueFilter::get_filter_from_matches(matches)?;
    let changes = IssueChanges {
        new_status: Some("muted".into()),
        ..Default::default()
    };

    if Api::current()
        .authenticated()?
        .bulk_update_issue(&org, &project, &filter, &changes)?
    {
        println!("Updated matching issues.");
        if let Some(status) = changes.new_status.as_ref() {
            println!("  new status: {status}");
        }
    } else {
        println!("No changes requested.");
    }
    Ok(())
}
