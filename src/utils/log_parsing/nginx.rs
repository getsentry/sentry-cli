use super::{LogEntry, LogFormat, LogLevel, LogParser};
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    /// Compiled regex for nginx combined log format
    /// Example: 192.168.1.1 - - [25/Dec/2023:10:00:00 +0000] "GET /index.html HTTP/1.1" 200 1024 "http://example.com" "Mozilla/5.0"
    static ref NGINX_COMBINED_REGEX: Regex = Regex::new(
        r#"^(\S+) \S+ \S+ \[([^\]]+)\] "([^"]*)" (\d+) (\S+) "([^"]*)" "([^"]*)""#
    ).expect("Invalid nginx combined regex");

    /// Compiled regex for nginx error log format
    /// Example: 2023/12/25 10:00:00 [error] 1234#0: *1 connect() failed (111: Connection refused)
    static ref NGINX_ERROR_REGEX: Regex = Regex::new(
        r#"^(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(\w+)\] \d+#\d+: (.+)$"#
    ).expect("Invalid nginx error regex");
}

/// Parser for nginx default log format
/// Default format: '$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent"'
pub struct NginxParser;

impl NginxParser {
    pub fn new() -> Self {
        NginxParser
    }

    /// Parse nginx combined/access log format
    fn parse_combined_log(&self, line: &str) -> Result<LogEntry> {
        if let Some(captures) = NGINX_COMBINED_REGEX.captures(line) {
            let mut fields = HashMap::new();

            // Extract fields
            fields.insert(
                "remote_addr".to_owned(),
                captures
                    .get(1)
                    .expect("regex capture group should exist")
                    .as_str()
                    .to_owned(),
            );

            let timestamp_str = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            let request = captures
                .get(3)
                .expect("regex capture group should exist")
                .as_str();
            let status = captures
                .get(4)
                .expect("regex capture group should exist")
                .as_str();
            let body_bytes = captures
                .get(5)
                .expect("regex capture group should exist")
                .as_str();
            let referer = captures
                .get(6)
                .expect("regex capture group should exist")
                .as_str();
            let user_agent = captures
                .get(7)
                .expect("regex capture group should exist")
                .as_str();

            fields.insert("request".to_owned(), request.to_owned());
            fields.insert("status".to_owned(), status.to_owned());
            fields.insert("body_bytes_sent".to_owned(), body_bytes.to_owned());

            if referer != "-" {
                fields.insert("http_referer".to_owned(), referer.to_owned());
            }
            if user_agent != "-" {
                fields.insert("http_user_agent".to_owned(), user_agent.to_owned());
            }

            // Parse timestamp - nginx format: 25/Dec/2023:10:00:00 +0000
            let timestamp = parse_nginx_timestamp(timestamp_str);

            // Determine log level based on HTTP status
            let level = match status.parse::<u16>().unwrap_or(200) {
                400..=499 => Some(LogLevel::Warning),
                500..=599 => Some(LogLevel::Error),
                _ => Some(LogLevel::Info),
            };

            Ok(LogEntry {
                message: line.to_owned(),
                timestamp,
                level,
                fields,
                format: LogFormat::Nginx,
            })
        } else {
            anyhow::bail!("Failed to parse nginx combined log line: {}", line);
        }
    }

    /// Parse nginx error log format
    fn parse_error_log(&self, line: &str) -> Result<LogEntry> {
        if let Some(captures) = NGINX_ERROR_REGEX.captures(line) {
            let mut fields = HashMap::new();

            let timestamp_str = captures
                .get(1)
                .expect("regex capture group should exist")
                .as_str();
            let level_str = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            let message = captures
                .get(3)
                .expect("regex capture group should exist")
                .as_str();

            fields.insert("error_message".to_owned(), message.to_owned());

            // Parse timestamp - nginx error format: 2023/12/25 10:00:00
            let timestamp = parse_nginx_error_timestamp(timestamp_str);

            // Parse log level
            let level = LogLevel::from_str(level_str).or({
                // nginx specific levels
                match level_str {
                    "emerg" | "alert" | "crit" => Some(LogLevel::Fatal),
                    "err" => Some(LogLevel::Error),
                    "warn" => Some(LogLevel::Warning),
                    "notice" | "info" => Some(LogLevel::Info),
                    "debug" => Some(LogLevel::Debug),
                    _ => None,
                }
            });

            Ok(LogEntry {
                message: line.to_owned(),
                timestamp,
                level,
                fields,
                format: LogFormat::Nginx,
            })
        } else {
            anyhow::bail!("Failed to parse nginx error log line: {}", line);
        }
    }
}

impl LogParser for NginxParser {
    fn parse_line(&self, line: &str) -> Result<LogEntry> {
        // Try combined format first, then error format
        if NGINX_COMBINED_REGEX.is_match(line) {
            self.parse_combined_log(line)
        } else if NGINX_ERROR_REGEX.is_match(line) {
            self.parse_error_log(line)
        } else {
            anyhow::bail!("Line does not match nginx log format: {}", line);
        }
    }

    fn can_parse(&self, line: &str) -> bool {
        NGINX_COMBINED_REGEX.is_match(line) || NGINX_ERROR_REGEX.is_match(line)
    }

    fn format(&self) -> LogFormat {
        LogFormat::Nginx
    }
}

/// Parse nginx access log timestamp: 25/Dec/2023:10:00:00 +0000
fn parse_nginx_timestamp(timestamp_str: &str) -> Option<DateTime<Utc>> {
    // Remove timezone part for parsing with chrono
    let without_tz = timestamp_str.split(' ').next()?;

    // Parse the datetime part
    NaiveDateTime::parse_from_str(without_tz, "%d/%b/%Y:%H:%M:%S")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

/// Parse nginx error log timestamp: 2023/12/25 10:00:00
fn parse_nginx_error_timestamp(timestamp_str: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(timestamp_str, "%Y/%m/%d %H:%M:%S")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nginx_combined_log() {
        let parser = NginxParser::new();
        let line = r#"192.168.1.1 - - [25/Dec/2023:10:00:00 +0000] "GET /index.html HTTP/1.1" 200 1024 "http://example.com" "Mozilla/5.0""#;

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Info)));
        assert_eq!(
            entry
                .fields
                .get("remote_addr")
                .expect("regex capture group should exist"),
            "192.168.1.1"
        );
        assert_eq!(
            entry
                .fields
                .get("status")
                .expect("regex capture group should exist"),
            "200"
        );
        assert!(matches!(entry.format, LogFormat::Nginx));
    }

    #[test]
    fn test_parse_nginx_error_log() {
        let parser = NginxParser::new();
        let line =
            "2023/12/25 10:00:00 [error] 1234#0: *1 connect() failed (111: Connection refused)";

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Error)));
        assert!(entry.fields.contains_key("error_message"));
        assert!(matches!(entry.format, LogFormat::Nginx));
    }

    #[test]
    fn test_can_parse_nginx_logs() {
        let parser = NginxParser::new();

        let combined_line = r#"192.168.1.1 - - [25/Dec/2023:10:00:00 +0000] "GET /index.html HTTP/1.1" 200 1024 "http://example.com" "Mozilla/5.0""#;
        assert!(parser.can_parse(combined_line));

        let error_line = "2023/12/25 10:00:00 [error] 1234#0: *1 connect() failed";
        assert!(parser.can_parse(error_line));

        let invalid_line = "This is not an nginx log";
        assert!(!parser.can_parse(invalid_line));
    }
}
