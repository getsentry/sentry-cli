use anyhow::Result;
use clap::Args;

use crate::api::{Api, Dataset, FetchEventsOptions};
use crate::config::Config;
use crate::utils::formatting::Table;

/// Fields to fetch from the logs API
const LOG_FIELDS: &[&str] = &[
    "sentry.item_id",
    "trace",
    "severity",
    "timestamp",
    "message",
];

/// Arguments for listing logs
#[derive(Args)]
pub(super) struct ListLogsArgs {
    #[arg(short = 'o', long = "org")]
    #[arg(help = "The organization ID or slug.")]
    org: Option<String>,

    #[arg(short = 'p', long = "project")]
    #[arg(help = "The project ID (slug not supported).")]
    project: Option<String>,

    #[arg(long = "max-rows", default_value = "100")]
    #[arg(help = "Maximum number of log entries to fetch and display (max 1000).")]
    max_rows: usize,

    #[arg(long = "query", default_value = "")]
    #[arg(help = "Query to filter logs. Example: \"level:error\"")]
    query: String,
}

pub(super) fn execute(args: ListLogsArgs) -> Result<()> {
    let config = Config::current();
    let (default_org, default_project) = config.get_org_and_project_defaults();

    let org = args
        .org
        .as_ref()
        .or(default_org.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!("No organization specified. Use --org or set a default in config.")
        })?
        .to_owned();
    let project = args
        .project
        .as_ref()
        .or(default_project.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!("No project specified. Use --project or set a default in config.")
        })?
        .to_owned();

    let api = Api::current();

    let query = if args.query.is_empty() {
        None
    } else {
        Some(args.query.as_str())
    };

    execute_single_fetch(&api, &org, &project, query, LOG_FIELDS, &args)
}

fn execute_single_fetch(
    api: &Api,
    org: &str,
    project: &str,
    query: Option<&str>,
    fields: &[&str],
    args: &ListLogsArgs,
) -> Result<()> {
    let options = FetchEventsOptions {
        dataset: Dataset::OurLogs,
        fields,
        project_id: Some(project),
        cursor: None,
        query,
        per_page: Some(args.max_rows),
        stats_period: Some("1h"),
        sort: Some("-timestamp"),
    };

    let logs = api
        .authenticated()?
        .fetch_organization_events(org, &options)?;

    let mut table = Table::new();
    table
        .title_row()
        .add("Item ID")
        .add("Timestamp")
        .add("Severity")
        .add("Message")
        .add("Trace");

    if let Some(logs) = logs.get(..args.max_rows) {
        for log in logs {
            let row = table.add_row();
            row.add(&log.item_id)
                .add(&log.timestamp)
                .add(log.severity.as_deref().unwrap_or(""))
                .add(log.message.as_deref().unwrap_or(""))
                .add(log.trace.as_deref().unwrap_or(""));
        }
    }

    if table.is_empty() {
        println!("No logs found");
    } else {
        table.print();
    }

    Ok(())
}
