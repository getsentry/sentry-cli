use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod apache;
pub mod nginx;
pub mod plain;

/// A structured log entry that can be sent to Sentry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// The original raw log line
    pub message: String,
    /// Parsed timestamp (if available)
    pub timestamp: Option<DateTime<Utc>>,
    /// Log level/severity (if detected)
    pub level: Option<LogLevel>,
    /// Additional structured fields extracted from the log
    pub fields: HashMap<String, String>,
    /// The format that was used to parse this entry
    pub format: LogFormat,
}

/// Log severity levels that map to Sentry levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl LogLevel {
    /// Convert to Sentry level string
    pub fn to_sentry_level(&self) -> &'static str {
        match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warning => "warning",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
        }
    }

    /// Parse log level from string (case insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" | "dbg" => Some(LogLevel::Debug),
            "info" | "information" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warning),
            "error" | "err" => Some(LogLevel::Error),
            "fatal" | "critical" | "crit" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
}

/// Supported log formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LogFormat {
    Nginx,
    Apache,
    Plain,
}

impl LogFormat {
    pub fn name(&self) -> &'static str {
        match self {
            LogFormat::Nginx => "nginx",
            LogFormat::Apache => "apache",
            LogFormat::Plain => "plain",
        }
    }
}

/// Trait for log format parsers
pub trait LogParser {
    /// Parse a single log line into a structured LogEntry
    fn parse_line(&self, line: &str) -> Result<LogEntry>;

    /// Check if this parser can handle the given log line
    /// Used for auto-detection
    fn can_parse(&self, line: &str) -> bool;

    /// Get the format name
    fn format(&self) -> LogFormat;
}

/// Auto-detect the log format based on sample lines
pub fn detect_log_format(sample_lines: &[String]) -> LogFormat {
    let parsers: Vec<Box<dyn LogParser>> = vec![
        Box::new(nginx::NginxParser::new()),
        Box::new(apache::ApacheParser::new()),
    ];

    // Count successful parses for each format
    let mut format_scores: HashMap<LogFormat, usize> = HashMap::new();

    for line in sample_lines.iter().take(10) {
        // Check first 10 lines
        if line.trim().is_empty() {
            continue;
        }

        for parser in &parsers {
            if parser.can_parse(line) {
                *format_scores.entry(parser.format()).or_insert(0) += 1;
            }
        }
    }

    // Return the format with the highest score, or Plain as fallback
    format_scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .map(|(format, _)| format)
        .unwrap_or(LogFormat::Plain)
}

/// Create a parser for the specified format
pub fn create_parser(format: LogFormat) -> Box<dyn LogParser> {
    match format {
        LogFormat::Nginx => Box::new(nginx::NginxParser::new()),
        LogFormat::Apache => Box::new(apache::ApacheParser::new()),
        LogFormat::Plain => Box::new(plain::PlainParser::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(
            LogLevel::from_str("ERROR").unwrap().to_sentry_level(),
            "error"
        );
        assert_eq!(
            LogLevel::from_str("info").unwrap().to_sentry_level(),
            "info"
        );
        assert_eq!(
            LogLevel::from_str("WARN").unwrap().to_sentry_level(),
            "warning"
        );
        assert!(LogLevel::from_str("invalid").is_none());
    }

    #[test]
    fn test_detect_log_format_fallback() {
        let sample_lines = vec!["some random text".to_owned()];
        assert!(matches!(detect_log_format(&sample_lines), LogFormat::Plain));
    }
}
