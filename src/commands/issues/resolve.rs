use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::api::{Api, IssueChanges, IssueFilter};
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command.about("Bulk resolve all selected issues.").arg(
        Arg::new("next_release")
            .long("next-release")
            .short('n')
            .action(ArgAction::SetTrue)
            .help("Only select issues in the next release."),
    )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let filter = IssueFilter::get_filter_from_matches(matches)?;
    let mut changes: IssueChanges = Default::default();

    if matches.get_flag("next_release") {
        changes.new_status = Some("resolvedInNextRelease".into());
    } else {
        changes.new_status = Some("resolved".into());
    }

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
