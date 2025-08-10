use super::{LogEntry, LogFormat, LogLevel, LogParser};
use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

/// Simple parser for plain text logs
/// Attempts to extract timestamps and log levels from unstructured text
pub struct PlainParser {
    /// Regex for detecting common timestamp patterns
    timestamp_regex: Regex,
    /// Regex for detecting log levels in text
    level_regex: Regex,
}

impl PlainParser {
    pub fn new() -> Self {
        // Common timestamp patterns
        // Matches: 2023-12-25 10:00:00, 2023/12/25 10:00:00, Dec 25 10:00:00, etc.
        let timestamp_pattern = r"(\d{4}[-/]\d{2}[-/]\d{2}[T\s]\d{2}:\d{2}:\d{2}|\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}|\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2})";
        let timestamp_regex = Regex::new(timestamp_pattern).expect("Invalid timestamp regex");

        // Log level detection - case insensitive
        let level_pattern = r"(?i)\b(TRACE|DEBUG|INFO|INFORMATION|WARN|WARNING|ERROR|ERR|FATAL|CRITICAL|CRIT|PANIC)\b";
        let level_regex = Regex::new(level_pattern).expect("Invalid level regex");

        PlainParser {
            timestamp_regex,
            level_regex,
        }
    }

    /// Extract timestamp from plain text
    fn extract_timestamp(&self, line: &str) -> Option<DateTime<Utc>> {
        if let Some(timestamp_match) = self.timestamp_regex.find(line) {
            let timestamp_str = timestamp_match.as_str();

            // Try different timestamp formats
            self.parse_timestamp_formats(timestamp_str)
        } else {
            None
        }
    }

    /// Try parsing various timestamp formats
    fn parse_timestamp_formats(&self, timestamp_str: &str) -> Option<DateTime<Utc>> {
        use chrono::NaiveDateTime;

        // List of common timestamp formats to try
        let formats = vec![
            "%Y-%m-%d %H:%M:%S", // 2023-12-25 10:00:00
            "%Y/%m/%d %H:%M:%S", // 2023/12/25 10:00:00
            "%Y-%m-%dT%H:%M:%S", // 2023-12-25T10:00:00
            "%d/%b/%Y:%H:%M:%S", // 25/Dec/2023:10:00:00
            "%b %d %H:%M:%S",    // Dec 25 10:00:00 (current year assumed)
        ];

        for format in formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(timestamp_str, format) {
                return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
            }
        }

        // Try with current year for formats without year
        use chrono::Datelike as _;
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(
            &format!("{} {timestamp_str}", chrono::Utc::now().year()),
            "%Y %b %d %H:%M:%S",
        ) {
            return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
        }

        None
    }

    /// Extract log level from plain text
    fn extract_level(&self, line: &str) -> Option<LogLevel> {
        if let Some(level_match) = self.level_regex.find(line) {
            LogLevel::from_str(level_match.as_str())
        } else {
            // Try to infer level from common keywords
            let line_lower = line.to_lowercase();
            if line_lower.contains("exception")
                || line_lower.contains("failed")
                || line_lower.contains("error")
            {
                Some(LogLevel::Error)
            } else if line_lower.contains("warning") || line_lower.contains("warn") {
                Some(LogLevel::Warning)
            } else {
                // Default to info for plain text
                Some(LogLevel::Info)
            }
        }
    }

    /// Extract structured fields from plain text using common patterns
    fn extract_fields(&self, line: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();

        // Extract key=value pairs
        let kv_regex = Regex::new(r"(\w+)=([^\s]+)").expect("Invalid KV regex");
        for captures in kv_regex.captures_iter(line) {
            let key = captures
                .get(1)
                .expect("regex capture group should exist")
                .as_str();
            let value = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            fields.insert(key.to_owned(), value.to_owned());
        }

        // Extract quoted strings that might be values
        let quoted_regex = Regex::new(r#""([^"]+)""#).expect("Invalid quoted regex");
        let mut quoted_values = Vec::new();
        for captures in quoted_regex.captures_iter(line) {
            if let Some(quoted) = captures.get(1) {
                quoted_values.push(quoted.as_str().to_owned());
            }
        }

        // Store quoted values if found
        for (i, value) in quoted_values.iter().enumerate() {
            fields.insert(format!("quoted_field_{i}"), value.clone());
        }

        fields
    }
}

impl LogParser for PlainParser {
    fn parse_line(&self, line: &str) -> Result<LogEntry> {
        let timestamp = self.extract_timestamp(line);
        let level = self.extract_level(line);
        let fields = self.extract_fields(line);

        Ok(LogEntry {
            message: line.to_owned(),
            timestamp,
            level,
            fields,
            format: LogFormat::Plain,
        })
    }

    fn can_parse(&self, _line: &str) -> bool {
        // Plain parser can handle any line as a fallback
        true
    }

    fn format(&self) -> LogFormat {
        LogFormat::Plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_log_with_timestamp() {
        let parser = PlainParser::new();
        let line = "2023-12-25 10:00:00 ERROR Something went wrong in the application";

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Error)));
        assert!(matches!(entry.format, LogFormat::Plain));
    }

    #[test]
    fn test_parse_plain_log_with_key_value() {
        let parser = PlainParser::new();
        let line = "User logged in successfully user_id=12345 session=abc123";

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(matches!(entry.level, Some(LogLevel::Info)));
        assert_eq!(
            entry
                .fields
                .get("user_id")
                .expect("regex capture group should exist"),
            "12345"
        );
        assert_eq!(
            entry
                .fields
                .get("session")
                .expect("regex capture group should exist"),
            "abc123"
        );
        assert!(matches!(entry.format, LogFormat::Plain));
    }

    #[test]
    fn test_parse_plain_log_with_quoted_values() {
        let parser = PlainParser::new();
        let line = r#"Processing request "GET /api/users" from client"#;

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(matches!(entry.level, Some(LogLevel::Info)));
        assert_eq!(
            entry
                .fields
                .get("quoted_field_0")
                .expect("regex capture group should exist"),
            "GET /api/users"
        );
        assert!(matches!(entry.format, LogFormat::Plain));
    }

    #[test]
    fn test_extract_timestamp() {
        let parser = PlainParser::new();

        assert!(parser
            .extract_timestamp("2023-12-25 10:00:00 message")
            .is_some());
        assert!(parser
            .extract_timestamp("2023/12/25 10:00:00 message")
            .is_some());
        assert!(parser
            .extract_timestamp("Dec 25 10:00:00 message")
            .is_some());
        assert!(parser.extract_timestamp("no timestamp here").is_none());
    }

    #[test]
    fn test_extract_level() {
        let parser = PlainParser::new();

        assert!(matches!(
            parser.extract_level("ERROR: something failed"),
            Some(LogLevel::Error)
        ));
        assert!(matches!(
            parser.extract_level("WARNING: be careful"),
            Some(LogLevel::Warning)
        ));
        assert!(matches!(
            parser.extract_level("INFO: normal operation"),
            Some(LogLevel::Info)
        ));
        assert!(matches!(
            parser.extract_level("DEBUG: detailed info"),
            Some(LogLevel::Debug)
        ));
        assert!(matches!(
            parser.extract_level("exception occurred"),
            Some(LogLevel::Error)
        ));
        assert!(matches!(
            parser.extract_level("plain message"),
            Some(LogLevel::Info)
        ));
    }

    #[test]
    fn test_can_parse_any_line() {
        let parser = PlainParser::new();

        assert!(parser.can_parse("Any line should be parseable"));
        assert!(parser.can_parse("Even empty lines"));
        assert!(parser.can_parse(""));
    }
}
