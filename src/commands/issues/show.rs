use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::utils::formatting::Table;
use crate::utils::logging::is_quiet_mode;
use crate::utils::system::QuietExit;

pub fn make_command(command: Command) -> Command {
    command.about("Show details of an issue.").arg(
        Arg::new("issue_id")
            .value_name("ISSUE_ID")
            .required(true)
            .help("The ID of the issue to show."),
    )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if matches.get_many::<String>("id").is_some()
        || matches.get_flag("all")
        || matches.get_one::<String>("status").is_some()
    {
        anyhow::bail!(
            "--id, --all, and --status are not supported by 'issues show'; \
             pass the issue ID directly as the ISSUE_ID argument"
        );
    }

    let api = Api::current();
    #[expect(clippy::unwrap_used, reason = "required arg")]
    let issue_id = matches.get_one::<String>("issue_id").unwrap();
    let issue = api.authenticated()?.get_issue(issue_id)?;

    if is_quiet_mode() {
        if issue.is_none() {
            return Err(QuietExit(1).into());
        }
        return Ok(());
    }

    match issue {
        None => anyhow::bail!("Issue {issue_id} not found"),
        Some(issue) => {
            let mut tbl = Table::new();
            tbl.add_row().add("Issue ID").add(&issue.id);
            tbl.add_row().add("Short ID").add(&issue.short_id);
            tbl.add_row().add("Title").add(&issue.title);
            tbl.add_row().add("Status").add(&issue.status);
            tbl.add_row().add("Level").add(&issue.level);
            tbl.add_row().add("First Seen").add(&issue.first_seen);
            tbl.add_row().add("Last Seen").add(&issue.last_seen);
            tbl.add_row().add("Events").add(&issue.count);
            tbl.add_row().add("Users").add(issue.user_count);
            tbl.add_row()
                .add("Culprit")
                .add(issue.culprit.as_deref().unwrap_or("-"));
            tbl.add_row()
                .add("Link")
                .add(issue.permalink.as_deref().unwrap_or("-"));
            tbl.print();
            Ok(())
        }
    }
}
