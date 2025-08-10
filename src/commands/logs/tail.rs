use anyhow::Result;
use clap::Args;
use log::{debug, info, warn};
use std::path::PathBuf;
use std::time::Duration;

use crate::utils::batching::{AdaptiveBatchingConfig, LogBatch};
use crate::utils::file_watcher::{
    setup_signal_handlers, FileEvent, LogFileWatcher, PositionTracker,
};
use crate::utils::log_parsing::{
    create_parser, detect_log_format, LogEntry, LogFormat as ParserLogFormat,
};
use crate::utils::log_transmission::{LogTransmitter, TransmissionRateLimiter};
use crate::utils::memory_monitor::{estimate_entry_size, MemoryMonitor};
use crate::utils::sampling::PrioritySampler;

/// Arguments for the tail logs command
#[derive(Args)]
pub(super) struct TailLogsArgs {
    #[arg(help = "Path to the log file to monitor")]
    file: PathBuf,

    #[arg(short = 'o', long = "org")]
    #[arg(help = "The organization ID or slug.")]
    org: Option<String>,

    #[arg(short = 'p', long = "project")]
    #[arg(help = "The project ID (slug not supported).")]
    project: Option<String>,

    #[arg(long = "batch-size", default_value = "100")]
    #[arg(help = "Number of log entries to batch before sending to Sentry (1-1000)")]
    #[arg(value_parser = clap::value_parser!(u32).range(1..=1000))]
    batch_size: u32,

    #[arg(long = "batch-timeout", default_value = "5")]
    #[arg(help = "Timeout in seconds for sending partial batches")]
    #[arg(value_parser = clap::value_parser!(u64).range(1..=300))]
    batch_timeout: u64,

    #[arg(long = "rate-limit", default_value = "1000")]
    #[arg(help = "Maximum number of log entries to send per minute")]
    #[arg(value_parser = clap::value_parser!(u32).range(1..=10000))]
    rate_limit: u32,

    #[arg(long = "memory-limit", default_value = "50")]
    #[arg(help = "Maximum memory usage in MB for buffering log entries")]
    memory_limit: usize,

    #[arg(long = "sampling-rate", default_value = "1.0")]
    #[arg(help = "Sampling rate for high-volume logs (0.0-1.0, 1.0 = no sampling)")]
    sampling_rate: f64,

    #[arg(long = "format")]
    #[arg(help = "Log format to parse (auto-detect if not specified)")]
    #[arg(value_enum)]
    format: Option<LogFormat>,
}

/// Supported log formats for parsing
#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum LogFormat {
    /// Auto-detect format based on file content
    #[default]
    Auto,
    /// nginx default log format
    Nginx,
    /// Apache Common Log Format  
    Apache,
    /// Plain text logs (fallback)
    Plain,
}

pub(super) fn execute(args: TailLogsArgs) -> Result<()> {
    use crate::config::Config;

    // Validate that the file exists and is readable
    if !args.file.exists() {
        anyhow::bail!("Log file does not exist: {}", args.file.display());
    }

    if !args.file.is_file() {
        anyhow::bail!("Path is not a file: {}", args.file.display());
    }

    // Validate organization and project configuration
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

    // Read file permissions to ensure we can read it
    std::fs::File::open(&args.file)
        .map_err(|e| anyhow::anyhow!("Cannot read log file {}: {}", args.file.display(), e))?;

    info!("Starting to tail log file: {}", args.file.display());
    info!("Organization: {org}");
    info!("Project: {project}");
    info!("Batch size: {}", args.batch_size);
    info!("Batch timeout: {}s", args.batch_timeout);
    info!(
        "Format: {:?}",
        args.format.as_ref().unwrap_or(&LogFormat::Auto)
    );

    // Set up signal handling for graceful shutdown
    let shutdown_receiver = setup_signal_handlers()?;

    // Initialize file watcher and position tracker
    let file_watcher = LogFileWatcher::new(&args.file)?;
    let mut position_tracker = PositionTracker::new(&args.file)?;

    // Initialize adaptive batching with memory awareness
    let adaptive_config = AdaptiveBatchingConfig {
        min_batch_size: 10,
        max_batch_size: (args.batch_size as usize * 2).min(1000),

        min_timeout: Duration::from_secs(1),
        max_timeout: Duration::from_secs(args.batch_timeout * 2),
        recent_flush_times: Vec::new(),
        max_history: 10,
    };

    let mut log_batch = LogBatch::new_adaptive(
        args.batch_size as usize,
        Duration::from_secs(args.batch_timeout),
        adaptive_config,
    );

    // Determine log format
    let log_format = match &args.format {
        Some(LogFormat::Auto) | None => {
            // Auto-detect format by reading first few lines
            info!("Auto-detecting log format...");
            let sample_lines = read_sample_lines(&args.file, 10)?;
            let detected_format = detect_log_format(&sample_lines);
            info!("Detected log format: {}", detected_format.name());
            detected_format
        }
        Some(LogFormat::Nginx) => ParserLogFormat::Nginx,
        Some(LogFormat::Apache) => ParserLogFormat::Apache,
        Some(LogFormat::Plain) => ParserLogFormat::Plain,
    };

    // Create parser for the determined format
    let parser = create_parser(log_format);

    // Initialize Sentry log transmitter
    let log_transmitter = LogTransmitter::new(org.clone(), project.clone())?;
    let mut rate_limiter = TransmissionRateLimiter::new(args.rate_limit);

    // Initialize performance monitoring and optimization features
    let memory_monitor = MemoryMonitor::new(args.memory_limit);
    let mut sampler = if args.sampling_rate < 1.0 {
        Some(PrioritySampler::new(args.rate_limit as f64 / 60.0)) // Convert per-minute to per-second
    } else {
        None
    };

    let poll_interval = Duration::from_millis(1000);

    info!("File monitoring started. Press Ctrl+C to stop.");

    loop {
        // Check for shutdown signal
        if shutdown_receiver.try_recv().is_ok() {
            info!("Shutdown signal received, flushing remaining logs...");

            // Flush any remaining log entries
            if !log_batch.is_empty() {
                let remaining_entries = log_batch.flush();
                info!("Flushing {} remaining log entries", remaining_entries.len());
                if let Err(e) =
                    send_log_entries(&log_transmitter, &mut rate_limiter, remaining_entries)
                {
                    warn!("Failed to send final batch: {}", e);
                }
            }

            break;
        }

        // Check for file system events
        match file_watcher.check_events(poll_interval)? {
            Some(FileEvent::DataWritten) => {
                // File has new data, read new lines
                let new_lines = position_tracker.read_new_lines()?;

                for line in new_lines {
                    debug!("New log line: {}", line);

                    // Check memory usage before processing
                    let entry_size = estimate_entry_size(&line);
                    if !memory_monitor.record_log_entry(entry_size) {
                        warn!("Memory limit exceeded, dropping log entry");
                        continue;
                    }

                    // Parse log format and create structured log entry
                    match parser.parse_line(&line) {
                        Ok(log_entry) => {
                            debug!("Parsed log entry: {:?}", log_entry);

                            // Apply sampling if configured
                            let should_process = if let Some(ref mut sampler) = sampler {
                                let level_str =
                                    log_entry.level.as_ref().map(|l| l.to_sentry_level());
                                sampler.should_sample(level_str)
                            } else {
                                true
                            };

                            if should_process {
                                let should_flush =
                                    log_batch.add_entry(serialize_log_entry(&log_entry)?);

                                if should_flush {
                                    let entries = log_batch.flush();
                                    info!(
                                        "Sending batch of {} log entries to Sentry",
                                        entries.len()
                                    );
                                    if let Err(e) = send_log_entries_with_monitoring(
                                        &log_transmitter,
                                        &mut rate_limiter,
                                        entries,
                                        &memory_monitor,
                                    ) {
                                        warn!("Failed to send log batch: {}", e);
                                    }
                                }
                            } else {
                                debug!("Entry dropped by sampling");
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse log line '{}': {}", line, e);
                            // Fall back to plain text
                            let should_process = if let Some(ref mut sampler) = sampler {
                                sampler.should_sample(Some("info")) // Default to info level for unparsed
                            } else {
                                true
                            };

                            if should_process {
                                let should_flush = log_batch.add_entry(line);

                                if should_flush {
                                    let entries = log_batch.flush();
                                    info!(
                                        "Sending batch of {} log entries to Sentry",
                                        entries.len()
                                    );
                                    if let Err(e) = send_log_entries_with_monitoring(
                                        &log_transmitter,
                                        &mut rate_limiter,
                                        entries,
                                        &memory_monitor,
                                    ) {
                                        warn!("Failed to send log batch: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Some(FileEvent::Deleted) => {
                warn!("Log file was deleted, stopping monitoring");
                break;
            }
            Some(FileEvent::Moved) => {
                warn!("Log file was moved/renamed, attempting to continue monitoring");
                // For log rotation, we might want to restart watching
            }
            Some(FileEvent::Created) => {
                info!("Log file was created/recreated");
                position_tracker = PositionTracker::new(&args.file)?;
            }
            None => {
                // No file events, check if batch should be flushed due to timeout
                if log_batch.should_flush() && !log_batch.is_empty() {
                    let entries = log_batch.flush();
                    info!(
                        "Timeout flush: sending batch of {} log entries to Sentry",
                        entries.len()
                    );
                    if let Err(e) = send_log_entries(&log_transmitter, &mut rate_limiter, entries) {
                        warn!("Failed to send timeout batch: {}", e);
                    }
                }
            }
        }

        // Periodic check for new data even without file system events
        let new_bytes = position_tracker.check_for_new_data()?;
        if new_bytes > 0 {
            let new_lines = position_tracker.read_new_lines()?;

            for line in new_lines {
                debug!("New log line (polling): {}", line);

                // Parse log format and create structured log entry
                match parser.parse_line(&line) {
                    Ok(log_entry) => {
                        debug!("Parsed log entry: {:?}", log_entry);
                        let should_flush = log_batch.add_entry(serialize_log_entry(&log_entry)?);

                        if should_flush {
                            let entries = log_batch.flush();
                            info!("Sending batch of {} log entries to Sentry", entries.len());
                            if let Err(e) =
                                send_log_entries(&log_transmitter, &mut rate_limiter, entries)
                            {
                                warn!("Failed to send log batch: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse log line '{}': {}", line, e);
                        // Fall back to plain text
                        let should_flush = log_batch.add_entry(line);

                        if should_flush {
                            let entries = log_batch.flush();
                            info!("Sending batch of {} log entries to Sentry", entries.len());
                            if let Err(e) =
                                send_log_entries(&log_transmitter, &mut rate_limiter, entries)
                            {
                                warn!("Failed to send log batch: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    info!("Log file monitoring stopped");
    Ok(())
}

/// Read sample lines from a file for format auto-detection
fn read_sample_lines(file_path: &PathBuf, max_lines: usize) -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::{BufRead as _, BufReader};

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    for line in reader.lines().take(max_lines) {
        let line = line?;
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    Ok(lines)
}

/// Serialize a LogEntry to JSON string for batching
fn serialize_log_entry(entry: &LogEntry) -> Result<String> {
    serde_json::to_string(entry)
        .map_err(|e| anyhow::anyhow!("Failed to serialize log entry: {}", e))
}

/// Send log entries to Sentry with rate limiting
fn send_log_entries(
    transmitter: &LogTransmitter,
    rate_limiter: &mut TransmissionRateLimiter,
    entries: Vec<String>,
) -> Result<()> {
    if !rate_limiter.can_send() {
        let (current, max) = rate_limiter.get_status();
        warn!(
            "Rate limit exceeded: {}/{} events this minute. Dropping {} log entries.",
            current,
            max,
            entries.len()
        );
        return Ok(());
    }

    match transmitter.send_log_batch(entries) {
        Ok(event_ids) => {
            debug!(
                "Successfully sent {} log entries to Sentry",
                event_ids.len()
            );
            Ok(())
        }
        Err(e) => {
            warn!("Failed to transmit logs to Sentry: {}", e);
            Err(e)
        }
    }
}

/// Send log entries to Sentry with rate limiting and memory monitoring
fn send_log_entries_with_monitoring(
    transmitter: &LogTransmitter,
    rate_limiter: &mut TransmissionRateLimiter,
    entries: Vec<String>,
    memory_monitor: &MemoryMonitor,
) -> Result<()> {
    // Calculate memory that will be freed
    let freed_bytes: usize = entries.iter().map(|e| estimate_entry_size(e)).sum();

    let result = send_log_entries(transmitter, rate_limiter, entries);

    // Update memory tracking after transmission
    memory_monitor.record_flush(freed_bytes);

    result
}
