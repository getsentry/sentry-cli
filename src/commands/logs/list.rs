use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use clap::Args;

use crate::api::{Api, Dataset, FetchEventsOptions, LogEntry};
use crate::config::Config;
use crate::utils::formatting::Table;

const MAX_ROWS_RANGE: std::ops::RangeInclusive<usize> = 1..=1000;
/// Validate that max_rows is in the allowed range
fn validate_max_rows(s: &str) -> Result<usize> {
    let value = s.parse()?;
    if MAX_ROWS_RANGE.contains(&value) {
        Ok(value)
    } else {
        Err(anyhow::anyhow!(
            "max-rows must be between {} and {}",
            MAX_ROWS_RANGE.start(),
            MAX_ROWS_RANGE.end()
        ))
    }
}

/// Validate that poll-interval is a positive integer (> 0)
fn validate_poll_interval(s: &str) -> Result<u64> {
    let value = s.parse()?;
    if value > 0 {
        Ok(value)
    } else {
        Err(anyhow::anyhow!("poll-interval must be a positive integer"))
    }
}

/// Check if a project identifier is numeric (project ID) or string (project slug)
fn is_numeric_project_id(project: &str) -> bool {
    !project.is_empty() && project.chars().all(|c| c.is_ascii_digit())
}

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
    #[arg(help = "The project ID or slug.")]
    project: Option<String>,

    #[arg(long = "max-rows", default_value = "100")]
    #[arg(value_parser = validate_max_rows)]
    #[arg(help = format!("Maximum number of log entries to fetch and display (max {}).", MAX_ROWS_RANGE.end()))]
    max_rows: usize,

    #[arg(long = "query", default_value = "")]
    #[arg(help = "Query to filter logs. Example: \"level:error\"")]
    query: String,

    #[arg(long = "live")]
    #[arg(help = "Live stream logs.")]
    live: bool,

    #[arg(long = "poll-interval", default_value = "2")]
    #[arg(value_parser = validate_poll_interval)]
    #[arg(help = "Poll interval in seconds (must be > 0). Only used when --live is specified.")]
    poll_interval: u64,
}

pub(super) fn execute(args: ListLogsArgs) -> Result<()> {
    let config = Config::current();
    let (default_org, default_project) = config.get_org_and_project_defaults();

    let org = args.org.as_ref().or(default_org.as_ref()).ok_or_else(|| {
        anyhow::anyhow!(
            "No organization specified. Please specify an organization using the --org argument."
        )
    })?;

    let project = args
        .project
        .as_ref()
        .or(default_project.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!("No project specified. Use --project or set a default in config.")
        })?;

    let api = Api::current();

    // Pass numeric project IDs as project parameter, otherwise pass as query string -
    // current API does not support project slugs as a parameter.
    let (query, project_id) = if is_numeric_project_id(project) {
        (Cow::Borrowed(&args.query), Some(project.as_str()))
    } else {
        let query = if args.query.is_empty() {
            format!("project:{project}")
        } else {
            format!("project:{project} {}", args.query)
        };
        (Cow::Owned(query), None)
    };

    if args.live {
        execute_live_streaming(&api, org, project_id, &query, LOG_FIELDS, &args)
    } else {
        execute_single_fetch(&api, org, project_id, &query, LOG_FIELDS, &args)
    }
}

fn execute_single_fetch(
    api: &Api,
    org: &str,
    project_id: Option<&str>,
    query: &str,
    fields: &[&str],
    args: &ListLogsArgs,
) -> Result<()> {
    let options = FetchEventsOptions {
        dataset: Dataset::Logs,
        fields,
        project_id,
        cursor: None,
        query,
        per_page: args.max_rows,
        stats_period: "90d",
        sort: "-timestamp",
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

    for log in logs.iter().take(args.max_rows) {
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

const MAX_DEDUP_BUFFER_SIZE: usize = 10_000;

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

/// Returns the updated consecutive all-new batch count and whether we should warn.
///
/// A batch is considered "all-new" if `fetched_count > 0` and `unique_count == fetched_count`.
/// - If the batch is all-new, the counter increments; when it reaches `threshold`, we reset it to 0 and return `should_warn = true`.
/// - If the batch is not all-new (including `fetched_count == 0`), the counter resets to 0 and `should_warn = false`.
fn evaluate_all_new_batch_state(
    previous_count: usize,
    fetched_count: usize,
    unique_count: usize,
    threshold: usize,
) -> (usize, bool) {
    let all_new_batch = fetched_count > 0 && unique_count == fetched_count;
    if all_new_batch {
        let updated = previous_count + 1;
        if updated >= threshold {
            (0, true)
        } else {
            (updated, false)
        }
    } else {
        (0, false)
    }
}

fn execute_live_streaming(
    api: &Api,
    org: &str,
    project: Option<&str>,
    query: &str,
    fields: &[&str],
    args: &ListLogsArgs,
) -> Result<()> {
    let mut deduplicator = LogDeduplicator::new(MAX_DEDUP_BUFFER_SIZE);
    let poll_duration = Duration::from_secs(args.poll_interval);
    let mut consecutive_new_only_count = 0;
    const WARNING_THRESHOLD: usize = 3; // Show message every 3 consecutive all-new polls

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
    // Holds a warning message to be printed after the current batch of rows for visibility
    let mut pending_warning: Option<String> = None;

    loop {
        let options = FetchEventsOptions {
            dataset: Dataset::Logs,
            fields,
            project_id: project,
            cursor: None,
            query,
            per_page: args.max_rows,
            stats_period: "10m",
            sort: "-timestamp",
        };

        match api
            .authenticated()?
            .fetch_organization_events(org, &options)
        {
            Ok(logs) => {
                let fetched_count = logs.len();
                let unique_logs = deduplicator.add_logs(logs);

                let (new_count, should_warn) = evaluate_all_new_batch_state(
                    consecutive_new_only_count,
                    fetched_count,
                    unique_logs.len(),
                    WARNING_THRESHOLD,
                );
                consecutive_new_only_count = new_count;
                if should_warn {
                    let suggestion_suffix = if args.query.trim().is_empty() {
                        Cow::Borrowed("")
                    } else {
                        Cow::Owned(format!(" (current filter: \"{}\")", args.query))
                    };
                    let msg = format!(
                        "Only new logs received in the last {WARNING_THRESHOLD} polls. You may be missing some logs. Consider narrowing your query filter{}.",
                        suggestion_suffix
                    );
                    pending_warning = Some(msg);
                }

                // Add new logs to table (if any)
                if !unique_logs.is_empty() {
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

                // Print any pending warning AFTER the batch rows to maximize visibility
                if let Some(msg) = pending_warning.take() {
                    // Style: bold black text on bright yellow background, with spacing and banner
                    const BANNER_WIDTH: usize = 100;
                    let line = "=".repeat(BANNER_WIDTH);
                    let reset = "\x1b[0m";
                    let style = "\x1b[30;103;1m"; // black on bright yellow, bold
                    eprintln!("\n\n{}\n{} {} {}\n{}\n\n", line, style, msg, reset, line);
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

    #[test]
    fn test_evaluate_all_new_batch_state_increments_and_warns() {
        let threshold = 3;
        // First all-new batch
        let (count1, warn1) = evaluate_all_new_batch_state(0, 5, 5, threshold);
        assert_eq!(count1, 1);
        assert!(!warn1);

        // Second all-new batch
        let (count2, warn2) = evaluate_all_new_batch_state(count1, 2, 2, threshold);
        assert_eq!(count2, 2);
        assert!(!warn2);

        // Third all-new batch should warn and reset
        let (count3, warn3) = evaluate_all_new_batch_state(count2, 10, 10, threshold);
        assert_eq!(count3, 0);
        assert!(warn3);

        // Non all-new batch resets
        let (count4, warn4) = evaluate_all_new_batch_state(2, 4, 3, threshold);
        assert_eq!(count4, 0);
        assert!(!warn4);

        // Empty fetch resets
        let (count5, warn5) = evaluate_all_new_batch_state(2, 0, 0, threshold);
        assert_eq!(count5, 0);
        assert!(!warn5);
    }

    #[test]
    fn test_is_numeric_project_id_purely_numeric() {
        assert!(is_numeric_project_id("123456"));
        assert!(is_numeric_project_id("1"));
        assert!(is_numeric_project_id("999999999"));
    }

    #[test]
    fn test_is_numeric_project_id_alphanumeric() {
        assert!(!is_numeric_project_id("abc123"));
        assert!(!is_numeric_project_id("123abc"));
        assert!(!is_numeric_project_id("my-project"));
    }

    #[test]
    fn test_is_numeric_project_id_numeric_with_dash() {
        assert!(!is_numeric_project_id("123-45"));
        assert!(!is_numeric_project_id("1-2-3"));
        assert!(!is_numeric_project_id("999-888"));
    }

    #[test]
    fn test_is_numeric_project_id_empty_string() {
        assert!(!is_numeric_project_id(""));
    }
}
