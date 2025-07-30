use anyhow::Result;
use clap::Args;

use crate::api::{Api, FetchEventsOptions};
use crate::config::Config;
use crate::utils::formatting::Table;

use super::common_args::CommonLogsArgs;

/// Arguments for listing logs
#[derive(Args)]
pub(super) struct ListLogsArgs {
    #[command(flatten)]
    pub(super) common: CommonLogsArgs,

    #[arg(long = "max-rows")]
    #[arg(help = "Maximum number of rows to print.")]
    pub(super) max_rows: Option<usize>,

    #[arg(long = "per-page", default_value = "100")]
    #[arg(help = "Number of log entries per request (max 1000).")]
    pub(super) per_page: usize,

    #[arg(long = "query", default_value = "")]
    #[arg(help = "Query to filter logs. Example: \"level:error\"")]
    pub(super) query: String,

    #[arg(long = "live")]
    #[arg(help = "Live-tail logs (not implemented yet).")]
    pub(super) live: bool,
}

pub(super) fn execute(args: ListLogsArgs) -> Result<()> {
    let config = Config::current();
    let (default_org, default_project) = config.get_org_and_project_defaults();

    let org = args.common.org.or(default_org).ok_or_else(|| {
        anyhow::anyhow!("No organization specified. Use --org or set a default in config.")
    })?;
    let project = args.common.project.or(default_project).ok_or_else(|| {
        anyhow::anyhow!("No project specified. Use --project or set a default in config.")
    })?;

    let api = Api::current();

    let query = if args.query.is_empty() {
        None
    } else {
        Some(args.query.as_str())
    };
    let fields = [
        "sentry.item_id",
        "trace",
        "severity",
        "timestamp",
        "message",
    ];

    let options = FetchEventsOptions {
        project_id: Some(&project),
        query,
        per_page: Some(args.per_page),
        stats_period: Some("1h"),
        ..Default::default()
    };

    let logs = api
        .authenticated()?
        .fetch_organization_events(&org, "ourlogs", &fields, options)?;

    let mut table = Table::new();
    table
        .title_row()
        .add("Item ID")
        .add("Timestamp")
        .add("Severity")
        .add("Message")
        .add("Trace");

    let max_rows = std::cmp::min(logs.len(), args.max_rows.unwrap_or(usize::MAX));

    if let Some(logs) = logs.get(..max_rows) {
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
