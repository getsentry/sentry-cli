use super::{LogEntry, LogFormat, LogLevel, LogParser};
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

/// Parser for Apache Common Log Format (CLF) and Combined Log Format
pub struct ApacheParser {
    /// Regex for Apache Common Log Format
    common_regex: Regex,
    /// Regex for Apache Combined Log Format (includes referer and user agent)
    combined_regex: Regex,
    /// Regex for Apache error logs
    error_regex: Regex,
}

impl ApacheParser {
    pub fn new() -> Self {
        // Apache Common Log Format: host ident authuser [timestamp] "request" status bytes
        // Example: 127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
        let common_pattern = r#"^(\S+) (\S+) (\S+) \[([^\]]+)\] "([^"]*)" (\d+) (\S+)$"#;
        let common_regex = Regex::new(common_pattern).expect("Invalid Apache common regex");

        // Apache Combined Log Format: common format + "referer" "user_agent"
        // Example: 127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326 "http://www.example.com/start.html" "Mozilla/4.08"
        let combined_pattern =
            r#"^(\S+) (\S+) (\S+) \[([^\]]+)\] "([^"]*)" (\d+) (\S+) "([^"]*)" "([^"]*)""#;
        let combined_regex = Regex::new(combined_pattern).expect("Invalid Apache combined regex");

        // Apache error log format
        // Example: [Wed Oct 11 14:32:52 2000] [error] [client 127.0.0.1] client denied by server configuration: /export/home/live/ap/htdocs/test
        let error_pattern = r#"^\[([^\]]+)\] \[(\w+)\] (?:\[client ([^\]]+)\] )?(.+)$"#;
        let error_regex = Regex::new(error_pattern).expect("Invalid Apache error regex");

        ApacheParser {
            common_regex,
            combined_regex,
            error_regex,
        }
    }

    /// Parse Apache Common Log Format
    fn parse_common_log(&self, line: &str) -> Result<LogEntry> {
        if let Some(captures) = self.common_regex.captures(line) {
            let mut fields = HashMap::new();

            // Extract fields
            let host = captures
                .get(1)
                .expect("regex capture group should exist")
                .as_str();
            let ident = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            let authuser = captures
                .get(3)
                .expect("regex capture group should exist")
                .as_str();
            let timestamp_str = captures
                .get(4)
                .expect("regex capture group should exist")
                .as_str();
            let request = captures
                .get(5)
                .expect("regex capture group should exist")
                .as_str();
            let status = captures
                .get(6)
                .expect("regex capture group should exist")
                .as_str();
            let bytes = captures
                .get(7)
                .expect("regex capture group should exist")
                .as_str();

            fields.insert("remote_host".to_owned(), host.to_owned());
            if ident != "-" {
                fields.insert("remote_ident".to_owned(), ident.to_owned());
            }
            if authuser != "-" {
                fields.insert("remote_user".to_owned(), authuser.to_owned());
            }
            fields.insert("request".to_owned(), request.to_owned());
            fields.insert("status".to_owned(), status.to_owned());
            if bytes != "-" {
                fields.insert("bytes_sent".to_owned(), bytes.to_owned());
            }

            // Parse timestamp - Apache format: 10/Oct/2000:13:55:36 -0700
            let timestamp = parse_apache_timestamp(timestamp_str);

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
                format: LogFormat::Apache,
            })
        } else {
            anyhow::bail!("Failed to parse Apache common log line: {}", line);
        }
    }

    /// Parse Apache Combined Log Format
    fn parse_combined_log(&self, line: &str) -> Result<LogEntry> {
        if let Some(captures) = self.combined_regex.captures(line) {
            let mut fields = HashMap::new();

            // Extract all fields from combined format
            let host = captures
                .get(1)
                .expect("regex capture group should exist")
                .as_str();
            let ident = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            let authuser = captures
                .get(3)
                .expect("regex capture group should exist")
                .as_str();
            let timestamp_str = captures
                .get(4)
                .expect("regex capture group should exist")
                .as_str();
            let request = captures
                .get(5)
                .expect("regex capture group should exist")
                .as_str();
            let status = captures
                .get(6)
                .expect("regex capture group should exist")
                .as_str();
            let bytes = captures
                .get(7)
                .expect("regex capture group should exist")
                .as_str();
            let referer = captures
                .get(8)
                .expect("regex capture group should exist")
                .as_str();
            let user_agent = captures
                .get(9)
                .expect("regex capture group should exist")
                .as_str();

            fields.insert("remote_host".to_owned(), host.to_owned());
            if ident != "-" {
                fields.insert("remote_ident".to_owned(), ident.to_owned());
            }
            if authuser != "-" {
                fields.insert("remote_user".to_owned(), authuser.to_owned());
            }
            fields.insert("request".to_owned(), request.to_owned());
            fields.insert("status".to_owned(), status.to_owned());
            if bytes != "-" {
                fields.insert("bytes_sent".to_owned(), bytes.to_owned());
            }
            if referer != "-" {
                fields.insert("http_referer".to_owned(), referer.to_owned());
            }
            if user_agent != "-" {
                fields.insert("http_user_agent".to_owned(), user_agent.to_owned());
            }

            let timestamp = parse_apache_timestamp(timestamp_str);

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
                format: LogFormat::Apache,
            })
        } else {
            anyhow::bail!("Failed to parse Apache combined log line: {}", line);
        }
    }

    /// Parse Apache error log format
    fn parse_error_log(&self, line: &str) -> Result<LogEntry> {
        if let Some(captures) = self.error_regex.captures(line) {
            let mut fields = HashMap::new();

            let timestamp_str = captures
                .get(1)
                .expect("regex capture group should exist")
                .as_str();
            let level_str = captures
                .get(2)
                .expect("regex capture group should exist")
                .as_str();
            let client = captures.get(3).map(|m| m.as_str());
            let message = captures
                .get(4)
                .expect("regex capture group should exist")
                .as_str();

            if let Some(client_ip) = client {
                fields.insert("client_ip".to_owned(), client_ip.to_owned());
            }
            fields.insert("error_message".to_owned(), message.to_owned());

            // Parse timestamp - Apache error format: Wed Oct 11 14:32:52 2000
            let timestamp = parse_apache_error_timestamp(timestamp_str);

            // Parse log level
            let level = LogLevel::from_str(level_str).or({
                // Apache specific levels
                match level_str {
                    "emerg" | "alert" | "crit" => Some(LogLevel::Fatal),
                    "error" => Some(LogLevel::Error),
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
                format: LogFormat::Apache,
            })
        } else {
            anyhow::bail!("Failed to parse Apache error log line: {}", line);
        }
    }
}

impl LogParser for ApacheParser {
    fn parse_line(&self, line: &str) -> Result<LogEntry> {
        // Try combined format first (more specific), then common, then error
        if self.combined_regex.is_match(line) {
            self.parse_combined_log(line)
        } else if self.common_regex.is_match(line) {
            self.parse_common_log(line)
        } else if self.error_regex.is_match(line) {
            self.parse_error_log(line)
        } else {
            anyhow::bail!("Line does not match Apache log format: {}", line);
        }
    }

    fn can_parse(&self, line: &str) -> bool {
        self.combined_regex.is_match(line)
            || self.common_regex.is_match(line)
            || self.error_regex.is_match(line)
    }

    fn format(&self) -> LogFormat {
        LogFormat::Apache
    }
}

/// Parse Apache access log timestamp: 10/Oct/2000:13:55:36 -0700
fn parse_apache_timestamp(timestamp_str: &str) -> Option<DateTime<Utc>> {
    // Remove timezone part for parsing with chrono
    let without_tz = timestamp_str.split(' ').next()?;

    // Parse the datetime part
    NaiveDateTime::parse_from_str(without_tz, "%d/%b/%Y:%H:%M:%S")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

/// Parse Apache error log timestamp: Wed Oct 11 14:32:52 2000
fn parse_apache_error_timestamp(timestamp_str: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(timestamp_str, "%a %b %d %H:%M:%S %Y")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apache_common_log() {
        let parser = ApacheParser::new();
        let line = r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Info)));
        assert_eq!(
            entry
                .fields
                .get("remote_host")
                .expect("regex capture group should exist"),
            "127.0.0.1"
        );
        assert_eq!(
            entry
                .fields
                .get("remote_user")
                .expect("regex capture group should exist"),
            "frank"
        );
        assert_eq!(
            entry
                .fields
                .get("status")
                .expect("regex capture group should exist"),
            "200"
        );
        assert!(matches!(entry.format, LogFormat::Apache));
    }

    #[test]
    fn test_parse_apache_combined_log() {
        let parser = ApacheParser::new();
        let line = r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326 "http://www.example.com/start.html" "Mozilla/4.08""#;

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Info)));
        assert_eq!(
            entry
                .fields
                .get("remote_host")
                .expect("regex capture group should exist"),
            "127.0.0.1"
        );
        assert_eq!(
            entry
                .fields
                .get("http_referer")
                .expect("regex capture group should exist"),
            "http://www.example.com/start.html"
        );
        assert_eq!(
            entry
                .fields
                .get("http_user_agent")
                .expect("regex capture group should exist"),
            "Mozilla/4.08"
        );
        assert!(matches!(entry.format, LogFormat::Apache));
    }

    #[test]
    fn test_parse_apache_error_log() {
        let parser = ApacheParser::new();
        let line = "[Wed Oct 11 14:32:52 2000] [error] [client 127.0.0.1] client denied by server configuration";

        let entry = parser
            .parse_line(line)
            .expect("regex capture group should exist");
        assert_eq!(entry.message, line);
        assert!(entry.timestamp.is_some());
        assert!(matches!(entry.level, Some(LogLevel::Error)));
        assert_eq!(
            entry
                .fields
                .get("client_ip")
                .expect("regex capture group should exist"),
            "127.0.0.1"
        );
        assert!(entry.fields.contains_key("error_message"));
        assert!(matches!(entry.format, LogFormat::Apache));
    }

    #[test]
    fn test_can_parse_apache_logs() {
        let parser = ApacheParser::new();

        let common_line = r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;
        assert!(parser.can_parse(common_line));

        let error_line = "[Wed Oct 11 14:32:52 2000] [error] client denied by server configuration";
        assert!(parser.can_parse(error_line));

        let invalid_line = "This is not an Apache log";
        assert!(!parser.can_parse(invalid_line));
    }
}
