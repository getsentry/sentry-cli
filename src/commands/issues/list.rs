use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command
        .about("List all issues in your organization.")
        .arg(
            Arg::new("max_rows")
                .long("max-rows")
                .value_name("MAX_ROWS")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of rows to print."),
        )
        .arg(
            Arg::new("pages")
                .long("pages")
                .value_name("PAGES")
                .default_value("5")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of pages to fetch (100 issues/page)."),
        )
        .arg(
            Arg::new("query")
                .long("query")
                .value_name("QUERY")
                .default_value("")
                .help("Query to pass at the request. An example is \"is:unresolved\""),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;
    let pages = *matches.get_one("pages").unwrap();
    let query = matches.get_one::<String>("query").cloned();
    let api = Api::current();

    let issues = api
        .authenticated()?
        .list_organization_project_issues(&org, &project, pages, query)?;

    let mut table = Table::new();
    table
        .title_row()
        .add("Issue ID")
        .add("Short ID")
        .add("Title")
        .add("Last seen")
        .add("Status")
        .add("Level");

    let max_rows = std::cmp::min(
        issues.len(),
        *matches.get_one("max_rows").unwrap_or(&usize::MAX),
    );

    if let Some(issues) = issues.get(..max_rows) {
        for issue in issues {
            let row = table.add_row();
            row.add(&issue.id)
                .add(&issue.short_id)
                .add(&issue.title)
                .add(&issue.last_seen)
                .add(&issue.status)
                .add(&issue.level);
        }
    }

    if table.is_empty() {
        println!("No issues found");
    } else {
        table.print();
    }

    Ok(())
}
