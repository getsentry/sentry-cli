use anyhow::Result;
use clap::Args;
use std::collections::HashSet;
use std::time::Duration;

use crate::api::{Api, Dataset, FetchEventsOptions, LogEntry};
use crate::config::Config;
use crate::utils::formatting::Table;

/// Validate that max_rows is greater than 0
fn validate_max_rows(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| "invalid number".to_owned())?;
    if value == 0 {
        Err("max-rows must be greater than 0".to_owned())
    } else {
        Ok(value)
    }
}

/// Fields to fetch from the logs API
const LOG_FIELDS: &[&str] = &[
    "sentry.item_id",
    "trace",
    "severity",
    "timestamp",
    "message",
];

/// Maximum number of log entries to keep in memory for deduplication
const MAX_DEDUP_BUFFER_SIZE: usize = 10_000;

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
    #[arg(value_parser = validate_max_rows)]
    #[arg(help = "Maximum number of log entries to fetch and display (max 1000).")]
    max_rows: usize,

    #[arg(long = "query", default_value = "")]
    #[arg(help = "Query to filter logs. Example: \"level:error\"")]
    query: String,

    #[arg(long = "live")]
    #[arg(help = "Enable live streaming mode to continuously poll for new logs.")]
    live: bool,

    #[arg(long = "poll-interval", default_value = "2")]
    #[arg(help = "Polling interval in seconds for live streaming mode.")]
    poll_interval: u64,
}

pub(super) fn execute(args: ListLogsArgs) -> Result<()> {
    let config = Config::current();
    let (default_org, default_project) = config.get_org_and_project_defaults();

    let org = args
        .org
        .as_ref()
        .or(default_org.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!("No organization specified. Please specify an organization using the --org argument.")
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

    if args.live {
        execute_live_streaming(&api, &org, &project, query, LOG_FIELDS, &args)
    } else {
        execute_single_fetch(&api, &org, &project, query, LOG_FIELDS, &args)
    }
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

    let logs_to_show = &logs[..args.max_rows.min(logs.len())];
    for log in logs_to_show {
        let row = table.add_row();
        row.add(&log.item_id)
            .add(&log.timestamp)
            .add(log.severity.as_deref().unwrap_or(""))
            .add(log.message.as_deref().unwrap_or(""))
            .add(log.trace.as_deref().unwrap_or(""));
    }

    if table.is_empty() {
        println!("No logs found");
    } else {
        table.print();
    }

    Ok(())
}

/// Manages deduplication of log entries with a bounded buffer
struct LogDeduplicator {
    /// Set of seen log IDs for quick lookup
    seen_ids: HashSet<String>,
    /// Buffer of log entries in order (for maintaining size limit)
    buffer: Vec<LogEntry>,
    /// Maximum size of the buffer
    max_size: usize,
}

impl LogDeduplicator {
    fn new(max_size: usize) -> Self {
        Self {
            seen_ids: HashSet::new(),
            buffer: Vec::new(),
            max_size,
        }
    }

    /// Add new logs and return only the ones that haven't been seen before
    fn add_logs(&mut self, new_logs: Vec<LogEntry>) -> Vec<LogEntry> {
        let mut unique_logs = Vec::new();

        for log in new_logs {
            if !self.seen_ids.contains(&log.item_id) {
                self.seen_ids.insert(log.item_id.clone());
                self.buffer.push(log.clone());
                unique_logs.push(log);
            }
        }

        // Maintain buffer size limit by removing oldest entries
        while self.buffer.len() > self.max_size {
            let removed_log = self.buffer.remove(0);
            self.seen_ids.remove(&removed_log.item_id);
        }

        unique_logs
    }
}

fn execute_live_streaming(
    api: &Api,
    org: &str,
    project: &str,
    query: Option<&str>,
    fields: &[&str],
    args: &ListLogsArgs,
) -> Result<()> {
    let mut deduplicator = LogDeduplicator::new(MAX_DEDUP_BUFFER_SIZE);
    let poll_duration = Duration::from_secs(args.poll_interval);
    let mut consecutive_new_only_count = 0;
    const WARNING_THRESHOLD: usize = 3; // Show warning after 3 consecutive new-only responses

    println!("Starting live log streaming...");
    println!(
        "Polling every {} seconds. Press Ctrl+C to stop.",
        args.poll_interval
    );

    // Set up table with headers and print header once
    let mut table = Table::new();
    table
        .title_row()
        .add("Item ID")
        .add("Timestamp")
        .add("Severity")
        .add("Message")
        .add("Trace");

    let mut header_printed = false;

    loop {
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

        match api
            .authenticated()?
            .fetch_organization_events(org, &options)
        {
            Ok(logs) => {
                let unique_logs = deduplicator.add_logs(logs);

                if unique_logs.is_empty() {
                    consecutive_new_only_count += 1;

                    if consecutive_new_only_count >= WARNING_THRESHOLD && args.query.is_empty() {
                        eprintln!(
                            "\n⚠️  Warning: No new logs found for {consecutive_new_only_count} consecutive polls."
                        );

                        // Reset counter to avoid spam
                        consecutive_new_only_count = 0;
                    }
                } else {
                    consecutive_new_only_count = 0;

                    // Add new logs to table
                    for log in unique_logs {
                        let row = table.add_row();
                        row.add(&log.item_id)
                            .add(&log.timestamp)
                            .add(log.severity.as_deref().unwrap_or(""))
                            .add(log.message.as_deref().unwrap_or(""))
                            .add(log.trace.as_deref().unwrap_or(""));
                    }

                    if !header_printed {
                        // Print header with first data batch so column widths match actual data
                        table.print_table_start();
                        header_printed = true;
                    } else {
                        // Print only the rows (without table borders) for subsequent batches
                        table.print_rows_only();
                    }
                    // Clear rows to free memory but keep the table structure for reuse
                    table.clear_rows();
                }
            }
            Err(e) => {
                eprintln!("Error fetching logs: {e}");
            }
        }

        std::thread::sleep(poll_duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_log(id: &str, message: &str) -> LogEntry {
        LogEntry {
            item_id: id.to_owned(),
            trace: None,
            severity: Some("info".to_owned()),
            timestamp: "2025-01-01T00:00:00Z".to_owned(),
            message: Some(message.to_owned()),
        }
    }

    #[test]
    fn test_log_deduplicator_new() {
        let deduplicator = LogDeduplicator::new(100);
        assert_eq!(deduplicator.seen_ids.len(), 0);
    }

    #[test]
    fn test_log_deduplicator_add_unique_logs() {
        let mut deduplicator = LogDeduplicator::new(10);

        let log1 = create_test_log("1", "test message 1");
        let log2 = create_test_log("2", "test message 2");

        let unique_logs = deduplicator.add_logs(vec![log1.clone(), log2.clone()]);

        assert_eq!(unique_logs.len(), 2);
        assert_eq!(deduplicator.seen_ids.len(), 2);
    }

    #[test]
    fn test_log_deduplicator_deduplicate_logs() {
        let mut deduplicator = LogDeduplicator::new(10);

        let log1 = create_test_log("1", "test message 1");
        let log2 = create_test_log("2", "test message 2");

        // Add logs first time
        let unique_logs1 = deduplicator.add_logs(vec![log1.clone(), log2.clone()]);
        assert_eq!(unique_logs1.len(), 2);

        // Add same logs again
        let unique_logs2 = deduplicator.add_logs(vec![log1.clone(), log2.clone()]);
        assert_eq!(unique_logs2.len(), 0); // Should be empty as logs already seen

        assert_eq!(deduplicator.seen_ids.len(), 2);
    }

    #[test]
    fn test_log_deduplicator_buffer_size_limit() {
        let mut deduplicator = LogDeduplicator::new(3);

        // Add 5 logs to a buffer with max size 3
        let logs = vec![
            create_test_log("1", "test message 1"),
            create_test_log("2", "test message 2"),
            create_test_log("3", "test message 3"),
            create_test_log("4", "test message 4"),
            create_test_log("5", "test message 5"),
        ];

        let unique_logs = deduplicator.add_logs(logs);
        assert_eq!(unique_logs.len(), 5);

        // After adding 5 logs to a buffer with max size 3, the oldest 2 should be evicted
        // So logs 1 and 2 should no longer be in the seen_ids set
        // Adding them again should return them as new logs
        let duplicate_logs = vec![
            create_test_log("1", "test message 1"),
            create_test_log("2", "test message 2"),
        ];
        let duplicate_unique_logs = deduplicator.add_logs(duplicate_logs);
        assert_eq!(duplicate_unique_logs.len(), 2);

        // Test that adding new logs still works
        let new_logs = vec![create_test_log("6", "test message 6")];
        let new_unique_logs = deduplicator.add_logs(new_logs);
        assert_eq!(new_unique_logs.len(), 1);
    }

    #[test]
    fn test_log_deduplicator_mixed_new_and_old_logs() {
        let mut deduplicator = LogDeduplicator::new(10);

        // Add initial logs
        let initial_logs = vec![
            create_test_log("1", "test message 1"),
            create_test_log("2", "test message 2"),
        ];
        let unique_logs1 = deduplicator.add_logs(initial_logs);
        assert_eq!(unique_logs1.len(), 2);

        // Add mix of new and old logs
        let mixed_logs = vec![
            create_test_log("1", "test message 1"), // old
            create_test_log("3", "test message 3"), // new
            create_test_log("2", "test message 2"), // old
            create_test_log("4", "test message 4"), // new
        ];
        let unique_logs2 = deduplicator.add_logs(mixed_logs);

        // Should only return the new logs (3 and 4)
        assert_eq!(unique_logs2.len(), 2);
        assert_eq!(unique_logs2[0].item_id, "3");
        assert_eq!(unique_logs2[1].item_id, "4");

        assert_eq!(deduplicator.seen_ids.len(), 4);
        assert_eq!(deduplicator.buffer.len(), 4);
    }
}
