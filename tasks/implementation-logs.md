# Logs Tail Feature Implementation Guide

This document provides a comprehensive guide to the logs tail feature implementation in sentry-cli. It's designed to help new developers understand the architecture, find their way around the codebase, and contribute to this feature.

## Overview

The logs tail feature provides real-time log file monitoring similar to `tail -f`, automatically parsing log entries and sending them to Sentry as structured events. It supports multiple log formats, adaptive performance optimizations, and handles high-volume scenarios gracefully.

## Architecture

The implementation follows a modular architecture with clear separation of concerns:

```
src/commands/logs/
├── mod.rs          # Command registration and subcommand routing
└── tail.rs         # Main tail command implementation

src/utils/
├── file_watcher/   # File monitoring and change detection
├── log_parsing/    # Log format parsing and auto-detection
├── batching.rs     # Intelligent batching with adaptive behavior
├── memory_monitor.rs # Memory usage tracking and limits
├── sampling.rs     # Priority-based and adaptive sampling
└── log_transmission.rs # Sentry API integration for log events
```

## Core Components

### 1. Command Structure (`src/commands/logs/`)

#### `mod.rs` - Command Registration
- Defines the `logs` subcommand with `LogsSubcommand` enum
- Registers the `tail` subcommand with appropriate help text
- Routes execution to `tail::execute()`

#### `tail.rs` - Main Implementation
The heart of the feature with these key responsibilities:

**Argument Parsing:**
```rust
pub struct TailLogsArgs {
    file: PathBuf,              // Log file to monitor
    org: Option<String>,        // Sentry organization
    project: Option<String>,    // Sentry project
    batch_size: u32,           // Entries per batch (default: 100)
    batch_timeout: u64,        // Batch timeout in seconds (default: 5)
    rate_limit: u32,           // Events per minute (default: 1000)
    memory_limit: usize,       // Memory limit in MB (default: 50)
    sampling_rate: f64,        // Sampling rate 0.0-1.0 (default: 1.0)
    format: Option<LogFormat>, // Log format (default: auto-detect)
}
```

**Main Event Loop:**
1. Initialize file watcher and position tracker
2. Set up adaptive batching and performance monitoring
3. Auto-detect log format from sample lines
4. Monitor file for changes with configurable polling
5. Parse new lines and apply sampling
6. Batch entries and transmit to Sentry
7. Handle file rotation, truncation, and other events

### 2. File Monitoring (`src/utils/file_watcher/`)

#### `mod.rs` - Cross-Platform File Watching
Uses the `notify` crate for efficient file system event monitoring:

**Key Components:**
- `FileEvent` enum: DataWritten, Truncated, Deleted, Moved, Created
- `LogFileWatcher`: Wraps `notify::RecommendedWatcher` with polling interface
- `setup_signal_handlers()`: Graceful shutdown with SIGINT/SIGTERM

**Usage Pattern:**
```rust
let file_watcher = LogFileWatcher::new(&file_path)?;
loop {
    match file_watcher.check_events(poll_interval)? {
        Some(FileEvent::DataWritten { .. }) => {
            // Handle new data
        }
        Some(FileEvent::Truncated { .. }) => {
            // Handle file truncation/rotation
        }
        // ... other events
    }
}
```

#### `position_tracker.rs` - File Position Management
Manages reading position within log files:

**Features:**
- Seeks to end of file on initialization (tail behavior)
- Tracks current position and file size
- Detects file truncation and rotation via inode/metadata changes
- Provides `read_new_lines()` for efficient incremental reading

### 3. Log Parsing (`src/utils/log_parsing/`)

#### `mod.rs` - Common Interface
Defines the parsing abstraction and auto-detection:

**Core Types:**
```rust
pub struct LogEntry {
    pub timestamp: Option<DateTime<Utc>>,
    pub level: Option<LogLevel>,
    pub message: String,
    pub fields: HashMap<String, String>,
}

pub trait LogParser {
    fn parse_line(&self, line: &str) -> Result<LogEntry>;
    fn can_parse(&self, line: &str) -> bool;
    fn format(&self) -> LogFormat;
}
```

**Auto-Detection:**
- `detect_log_format()` analyzes sample lines from the file
- Tries each parser and scores based on successful parsing
- Falls back to plain text parser if no format matches

#### Format-Specific Parsers

**`nginx.rs`** - nginx Log Parser
- Supports combined access log format and error log format
- Uses `lazy_static!` for optimized regex compilation
- Extracts IP, request, status, bytes, referrer, user agent, etc.

**`apache.rs`** - Apache Log Parser
- Handles Common Log Format and Combined Log Format
- Supports Apache error log format
- Similar optimization with pre-compiled regex patterns

**`plain.rs`** - Plain Text Parser
- Fallback parser for unstructured logs
- Attempts to extract timestamps and log levels
- Handles key-value pair extraction

### 4. Performance Optimizations

#### `batching.rs` - Adaptive Batching
Intelligent batching that adapts to log volume:

**Adaptive Configuration:**
```rust
pub struct AdaptiveBatchingConfig {
    pub min_batch_size: usize,     // Minimum entries per batch
    pub max_batch_size: usize,     // Maximum entries per batch
    pub base_timeout: Duration,    // Base batching timeout
    pub min_timeout: Duration,     // Minimum timeout (high volume)
    pub max_timeout: Duration,     // Maximum timeout (low volume)
    // ... tracking fields
}
```

**Behavior:**
- Increases batch size during high-volume periods for efficiency
- Decreases batch size during low-volume periods for responsiveness
- Adjusts timeouts based on recent flush patterns

#### `memory_monitor.rs` - Memory Management
Tracks and enforces memory usage limits:

**Features:**
- `MemoryMonitor`: Tracks current usage vs. configurable limits
- `estimate_entry_size()`: Estimates memory footprint of log entries
- `BoundedLogQueue`: Memory-bounded queue with automatic cleanup
- Periodic memory statistics logging

#### `sampling.rs` - Intelligent Sampling
Priority-based sampling for high-volume scenarios:

**Sampling Strategies:**
- `AdaptiveSampler`: Adjusts rate based on target events per second
- `PrioritySampler`: Different rates per log level (fatal=100%, debug=1%)
- `hash_sample()`: Deterministic sampling based on content hash

**Log Level Priorities:**
```rust
fatal   -> 100% sampling (always include)
error   -> 80% sampling
warning -> 40% sampling
info    -> 10% sampling
debug   -> 1% sampling
```

### 5. Sentry Integration (`src/utils/log_transmission.rs`)

#### LogTransmitter
Handles conversion and transmission of log entries to Sentry using the official Sentry logs protocol:

**Key Methods:**
- `send_log_batch()`: Converts log entries to proper Sentry log envelopes (not events)
- `convert_to_sentry_log()`: Maps LogEntry to SentryLogPayload structure
- `create_structured_sentry_log()` / `create_plain_text_sentry_log()`: Creates protocol-compliant log payloads
- `send_raw_envelope()`: Sends raw envelope bytes using proper logs protocol

#### Sentry Logs Protocol Compliance
The implementation follows the [official Sentry logs protocol specification](https://develop.sentry.dev/sdk/telemetry/logs/):

**Correct Envelope Structure:**
```json
{}  // Empty header (no event_id for logs)
{"type":"log","item_count":N,"content_type":"application/vnd.sentry.items.log+json","length":X}
{"items":[{...log1...}, {...log2...}, {...logN...}]}
```

**SentryLogPayload Structure:**
```rust
pub struct SentryLogPayload {
    pub timestamp: f64,           // Unix timestamp
    pub trace_id: String,         // 32-char hex trace ID
    pub level: String,            // Log level (debug, info, warn, error, fatal)
    pub body: String,             // Log message
    pub attributes: HashMap<String, SentryLogAttribute>,  // Structured data
}
```

**Key Features:**
- Logs appear in Sentry's **Logs section** (not Events section)
- Each log gets a unique trace ID for correlation
- Structured attributes preserve parsed log fields
- Proper severity levels with numeric mapping
- Batch transmission for efficiency

#### Updated EnvelopesApi
Added `send_raw_envelope()` method to `src/api/envelopes_api.rs` for sending raw envelope bytes while maintaining authentication and error handling.

## Key Implementation Patterns

### 1. Error Handling
Uses `anyhow::Result` throughout for ergonomic error handling:
```rust
use anyhow::{Context, Result};

fn process_file() -> Result<()> {
    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    // ...
}
```

### 2. Performance Optimizations
- **Regex Compilation:** Uses `lazy_static!` to compile regex patterns once
- **Memory Management:** Tracks usage and enforces limits to prevent OOM
- **Adaptive Behavior:** Adjusts batching and sampling based on actual load
- **Efficient I/O:** Uses file position tracking for incremental reading

### 3. Configuration
Follows sentry-cli patterns for configuration:
- Command-line arguments with sensible defaults
- Validation and range checking for numeric parameters
- Optional parameters with fallback behavior

### 4. Testing Strategy
The implementation includes comprehensive testing:

**Unit Tests:**
- Placed alongside source files (e.g., in `sampling.rs`, `memory_monitor.rs`)
- Test individual component behavior and edge cases
- Protocol compliance tests in `log_transmission.rs`

**Integration Tests:**
- Located in `tests/integration/logs/`
- Use `.trycmd` format following sentry-cli conventions
- Test end-to-end command behavior

**Protocol Compliance Verification:**
- Real-time testing with actual Sentry DSN
- Verification via `sentry-cli logs list` command
- Debug output validation for proper envelope format
- Confirmation that logs appear in Sentry Logs section (not Events)

## Usage Examples

### Basic Usage
```bash
# Monitor nginx access log with auto-detection
sentry-cli logs tail /var/log/nginx/access.log --org my-org --project my-project

# Monitor with specific format and custom batching
sentry-cli logs tail app.log --format nginx --batch-size 50 --batch-timeout 10

# High-volume scenario with sampling and memory limits
sentry-cli logs tail high-volume.log --sampling-rate 0.1 --memory-limit 100 --rate-limit 2000
```

### Performance Tuning

**Low Volume Logs (< 10 entries/minute):**
```bash
--batch-size 10 --batch-timeout 30 --sampling-rate 1.0
```

**Medium Volume Logs (100-1000 entries/minute):**
```bash
--batch-size 100 --batch-timeout 5 --sampling-rate 1.0 --memory-limit 50
```

**High Volume Logs (> 1000 entries/minute):**
```bash
--batch-size 500 --batch-timeout 2 --sampling-rate 0.2 --memory-limit 200 --rate-limit 2000
```

## Development Guidelines

### Adding New Log Formats

1. **Create Parser Module:** Add new file in `src/utils/log_parsing/`
2. **Implement LogParser Trait:** Provide `parse_line()`, `can_parse()`, and `format()` methods
3. **Update Format Enum:** Add new variant to `LogFormat` in `mod.rs`
4. **Register Parser:** Update `create_parser()` function
5. **Add Tests:** Include unit tests and sample log fixtures

### Extending Performance Features

1. **Memory Monitoring:** Extend `MemoryMonitor` for additional metrics
2. **Sampling Strategies:** Add new sampling algorithms to `sampling.rs`
3. **Adaptive Behavior:** Enhance `AdaptiveBatchingConfig` with new parameters
4. **Metrics Collection:** Add performance metrics collection and reporting

### Debugging and Troubleshooting

**Enable Debug Logging:**
```bash
sentry-cli logs tail --log-level debug /path/to/logfile
```

**Verify Logs Ingestion:**
```bash
# Check if logs are being sent (should show HTTP 200 responses)
sentry-cli logs tail /path/to/logfile --log-level debug --org ORG --project PROJECT

# Verify logs appear in Sentry (requires auth token)
sentry-cli logs list --org ORG --project PROJECT --max-rows 10
```

**Expected Debug Output:**
- File watcher initialization and event detection
- Log parsing results with extracted fields
- Envelope structure showing correct protocol format
- HTTP requests/responses for log transmission
- Batch processing statistics

**Common Issues:**
- **File Permission Errors:** Ensure sentry-cli has read access to log files
- **Memory Pressure:** Increase `--memory-limit` or decrease `--batch-size`
- **Rate Limiting:** Adjust `--rate-limit` based on Sentry plan limits
- **Parse Failures:** Check log format or use `--format plain` as fallback
- **Logs Not Appearing:** Verify DSN/auth token, check debug output for HTTP errors

## Protocol Compliance & Standards

### Sentry Logs Protocol
The implementation strictly follows the [Sentry Logs Protocol](https://develop.sentry.dev/sdk/telemetry/logs/):

**Critical Requirements:**
- Logs must use `type: "log"` envelope items (not `type: "event"`)
- Envelope header must be empty `{}` (no event_id for logs)
- Content-Type must be `application/vnd.sentry.items.log+json`
- Logs must be wrapped in `{"items": [...]}` structure
- Each log needs unique trace_id and proper timestamp
- Severity levels must map to Sentry's expected values

**Verification:**
- Logs appear in Sentry's "Logs" section (not "Issues/Events")
- Debug output shows correct envelope format
- HTTP responses return 200 OK (not empty `{}` body)

### Performance Standards
- **Memory Efficiency:** Default 50MB limit with monitoring
- **Batch Processing:** Adaptive batching (10-500 entries per batch)
- **Rate Limiting:** Configurable events per minute
- **Error Handling:** Graceful degradation on parse failures

## Future Enhancement Ideas

1. **Additional Log Formats:** Support for JSON logs, custom formats via regex
2. **Real-time Metrics:** Expose parsing/transmission metrics via HTTP endpoint
3. **Multiple File Support:** Monitor multiple log files simultaneously
4. **Cloud Integration:** Direct integration with cloud logging services
5. **Advanced Sampling:** Machine learning-based anomaly detection for sampling

## Dependencies

Key external crates used in the implementation:

- `notify`: Cross-platform file system event monitoring
- `ctrlc`: Signal handling for graceful shutdown
- `regex`: Pattern matching for log parsing
- `lazy_static`: Compile-time regex optimization
- `chrono`: Date and time handling
- `serde/serde_json`: Serialization for Sentry events
- `rand`: Random number generation for sampling
- `anyhow`: Error handling and context

## Related Files

**Command Integration:**
- `src/commands/mod.rs`: Top-level command registration
- `src/commands/derive_parser.rs`: CLI parser definition

**API Integration:**
- `src/api/envelopes_api.rs`: Sentry envelopes API
- `src/api/mod.rs`: Core API client

**Testing:**
- `tests/integration/logs/`: Integration test suite
- `tests/integration/_fixtures/logs/`: Test log files
- `tests/integration/_cases/logs/`: Test cases in `.trycmd` format

This implementation provides a solid foundation for real-time log monitoring with Sentry integration, balancing performance, reliability, and usability.
