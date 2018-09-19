//! Implements a command for issue management.
use clap::{App, AppSettings, ArgMatches};
use failure::{Error, ResultExt};

use api::{Api, IssueChanges, IssueFilter};
use config::Config;
use utils::args::{validate_org, validate_project};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (setting: AppSettings::SubcommandRequiredElseHelp)
        (about: "Manage issues in Sentry.")
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} "The project slug.")
        (@arg status: -s --status [STATUS] possible_values(&["resolved", "muted", "unresolved"])
            "Select all issues matching a given status.")
        (@arg all: -a --all "Select all issues (this might be limited).")
        (@arg id: -i --id [ID]... "Select the issue with the given ID.")
        (@subcommand resolve =>
            (about: "Bulk resolve all selected issues.")
            (@arg next_release: -n --next-releases "Only select issues in the next release.")
        )
        (@subcommand mute =>
            (about: "Bulk mute all selected issues.")
        )
        (@subcommand unresolve =>
            (about: "Bulk unresolve all selected issues.")
        )
    )
}

fn get_filter_from_matches<'a>(matches: &ArgMatches<'a>) -> Result<IssueFilter, Error> {
    if matches.is_present("all") {
        return Ok(IssueFilter::All);
    }
    if let Some(status) = matches.value_of("status") {
        return Ok(IssueFilter::Status(status.into()));
    }
    let mut ids = vec![];
    if let Some(values) = matches.values_of("id") {
        for value in values {
            ids.push(value.parse::<u64>().context("Invalid issue ID")?);
        }
    }

    if ids.is_empty() {
        Ok(IssueFilter::Empty)
    } else {
        Ok(IssueFilter::ExplicitIds(ids))
    }
}

fn execute_change(
    org: &str,
    project: &str,
    filter: &IssueFilter,
    changes: &IssueChanges,
) -> Result<(), Error> {
    if Api::get_current().bulk_update_issue(org, project, filter, changes)? {
        println!("Updated matching issues.");
        if let Some(status) = changes.new_status.as_ref() {
            println!("  new status: {}", status);
        }
    } else {
        println!("No changes requested.");
    }
    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;
    let filter = get_filter_from_matches(matches)?;
    let mut changes: IssueChanges = Default::default();

    if let Some(sub_matches) = matches.subcommand_matches("resolve") {
        if sub_matches.is_present("next_release") {
            changes.new_status = Some("resolvedInNextRelease".into());
        } else {
            changes.new_status = Some("resolved".into());
        }
    } else if matches.subcommand_matches("mute").is_some() {
        changes.new_status = Some("muted".into());
    } else if matches.subcommand_matches("unresolve").is_some() {
        changes.new_status = Some("unresolved".into());
    }

    execute_change(&org, &project, &filter, &changes)
}
