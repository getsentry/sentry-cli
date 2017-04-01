//! Implements a command for issue management.
use crates::clap::{App, AppSettings, Arg, ArgMatches};

use prelude::*;
use api::{Api, IssueFilter, IssueChanges};
use config::Config;
use utils::ArgExt;


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("manage issues in Sentry")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_project_args()
        .arg(Arg::with_name("status")
            .long("status")
            .short("s")
            .value_name("STATUS")
            .possible_values(&["resolved", "muted", "unresolved"])
            .help("Only changes issues with this status"))
        .arg(Arg::with_name("all")
            .long("all")
            .short("a")
            .help("Selects all issues (this might be limited)"))
        .arg(Arg::with_name("id")
            .multiple(true)
            .short("i")
            .long("id")
            .help("Explicit issue IDs to resolve"))
        .subcommand(App::new("resolve")
            .about("Bulk resolve all matching issues")
            .arg(Arg::with_name("next_release")
                .long("next-release")
                .short("n")
                .help("Resolve in next release only")))
        .subcommand(App::new("mute").about("Bulk mute all matching issues"))
        .subcommand(App::new("unresolve").about("Bulk unresolve all matching issues"))
}

fn get_filter_from_matches<'a>(matches: &ArgMatches<'a>) -> Result<IssueFilter> {
    if matches.is_present("all") {
        return Ok(IssueFilter::All);
    }
    if let Some(status) = matches.value_of("status") {
        return Ok(IssueFilter::Status(status.into()));
    }
    let mut ids = vec![];
    if let Some(values) = matches.values_of("id") {
        for value in values {
            ids.push(value.parse().chain_err(|| "Invalid issue ID")?);
        }
    }

    if ids.is_empty() {
        Ok(IssueFilter::Empty)
    } else {
        Ok(IssueFilter::ExplicitIds(ids))
    }
}

fn execute_change(config: &Config,
                  org: &str,
                  project: &str,
                  filter: &IssueFilter,
                  changes: &IssueChanges)
                  -> Result<()> {
    if Api::new(config).bulk_update_issue(org, project, filter, changes)? {
        println!("Updated matching issues.");
        if let Some(status) = changes.new_status.as_ref() {
            println!("  new status: {}", status);
        }
    } else {
        println!("No changes requested.");
    }
    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let (org, project) = config.get_org_and_project(matches)?;
    let filter = get_filter_from_matches(matches)?;
    let mut changes: IssueChanges = Default::default();

    if let Some(sub_matches) = matches.subcommand_matches("resolve") {
        if sub_matches.is_present("next_release") {
            changes.new_status = Some("resolvedInNextRelease".into());
        } else {
            changes.new_status = Some("resolved".into());
        }
    } else if let Some(_) = matches.subcommand_matches("mute") {
        changes.new_status = Some("muted".into());
    } else if let Some(_) = matches.subcommand_matches("unresolve") {
        changes.new_status = Some("unresolved".into());
    }

    return execute_change(config, &org, &project, &filter, &changes);
}
