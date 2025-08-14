use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use log::debug;

use sentry::types::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::envelopes_api::EnvelopesApi;
use crate::utils::log_parsing::{LogEntry, LogLevel};

/// Sentry log payload as per the official logs protocol
/// https://develop.sentry.dev/sdk/telemetry/logs/#log-envelope-item-payload
#[derive(Debug, Serialize, Deserialize)]
pub struct SentryLogPayload {
    /// The timestamp of the log in seconds since the Unix epoch
    pub timestamp: f64,
    /// The trace id of the log (16 random bytes encoded as hex string)
    pub trace_id: String,
    /// The severity level of the log
    pub level: String,
    /// The log body/message
    pub body: String,
    /// Dictionary of key-value pairs with typed values
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, SentryLogAttribute>,
    /// Optional severity number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity_number: Option<u8>,
}

/// Attribute value with type information as per Sentry logs protocol
#[derive(Debug, Serialize, Deserialize)]
pub struct SentryLogAttribute {
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub attr_type: String,
}

/// Envelope payload for log items as per Sentry specification
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEnvelopeItems {
    pub items: Vec<SentryLogPayload>,
}

/// Converts our parsed log entries into proper Sentry log envelopes and sends them
pub struct LogTransmitter {
    envelope_api: EnvelopesApi,
    org: String,
    project: String,
}

impl LogTransmitter {
    /// Create a new log transmitter
    pub fn new(org: String, project: String) -> Result<Self> {
        let envelope_api = EnvelopesApi::try_new()
            .context("Failed to initialize Sentry envelope API. Check your DSN configuration.")?;

        Ok(LogTransmitter {
            envelope_api,
            org,
            project,
        })
    }

    /// Send a batch of log entries to Sentry using the proper logs protocol
    pub fn send_log_batch(&self, log_entries: Vec<String>) -> Result<Vec<Uuid>> {
        if log_entries.is_empty() {
            return Ok(Vec::new());
        }

        let mut sentry_logs = Vec::new();

        for log_entry_json in log_entries {
            let sentry_log = self.convert_to_sentry_log(&log_entry_json);
            sentry_logs.push(sentry_log);
        }

        if sentry_logs.is_empty() {
            return Ok(Vec::new());
        }

        // Send all logs in a single envelope with proper specification format
        // https://develop.sentry.dev/sdk/telemetry/logs/#appendix-a-example-log-envelope
        let envelope_payload = LogEnvelopeItems { items: sentry_logs };

        let envelope_json = serde_json::to_string(&envelope_payload)
            .context("Failed to serialize log envelope payload")?;

        // Create proper log envelope according to specification
        let envelope_header = "{}"; // Empty header for logs (no event_id needed)
        let item_header = format!(
            r#"{{"type":"log","item_count":{},"content_type":"application/vnd.sentry.items.log+json"}}"#,
            envelope_payload.items.len()
        );
        let envelope_content = format!("{envelope_header}\n{item_header}\n{envelope_json}");

        debug!("Sending log envelope:\n{}", envelope_content);

        // Send the envelope using the raw API
        self.send_raw_envelope(envelope_content.into_bytes())
            .context("Failed to send log envelope")?;

        debug!(
            "Successfully sent {} log entries to Sentry",
            envelope_payload.items.len()
        );
        Ok(vec![Uuid::new_v4()]) // Return single ID for the envelope
    }

    /// Convert a log entry JSON string to Sentry log format
    fn convert_to_sentry_log(&self, log_entry_json: &str) -> SentryLogPayload {
        // First, try to deserialize as our LogEntry type
        match serde_json::from_str::<LogEntry>(log_entry_json) {
            Ok(log_entry) => self.create_structured_sentry_log(&log_entry),
            Err(_) => {
                // Fall back to treating it as plain text
                self.create_plain_text_sentry_log(log_entry_json)
            }
        }
    }

    /// Create a structured Sentry log from our LogEntry
    fn create_structured_sentry_log(&self, log_entry: &LogEntry) -> SentryLogPayload {
        let mut attributes = HashMap::new();

        // Add structured fields as attributes with proper typing
        for (key, value) in &log_entry.fields {
            attributes.insert(
                key.clone(),
                SentryLogAttribute {
                    value: serde_json::Value::String(value.clone()),
                    attr_type: "string".to_owned(),
                },
            );
        }

        // Add SDK information as per spec
        attributes.insert(
            "sentry.sdk.name".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String("sentry-cli".to_owned()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "sentry.sdk.version".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(env!("CARGO_PKG_VERSION").to_owned()),
                attr_type: "string".to_owned(),
            },
        );

        // Add format information
        attributes.insert(
            "log_format".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(log_entry.format.name().to_owned()),
                attr_type: "string".to_owned(),
            },
        );

        // Add source information
        attributes.insert(
            "log_source".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String("sentry-cli-tail".to_owned()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "organization".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(self.org.clone()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "project".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(self.project.clone()),
                attr_type: "string".to_owned(),
            },
        );

        // Get timestamp
        let timestamp = if let Some(ts) = log_entry.timestamp {
            ts.timestamp() as f64 + (ts.timestamp_subsec_nanos() as f64 / 1_000_000_000.0)
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        };

        // Generate a random trace_id (32 hex chars)
        let trace_id = format!("{:032x}", rand::random::<u128>());

        let (level, severity_number) = convert_log_level_to_sentry(&log_entry.level);

        SentryLogPayload {
            timestamp,
            trace_id,
            level,
            body: log_entry.message.clone(),
            attributes,
            severity_number: Some(severity_number),
        }
    }

    /// Create a plain text Sentry log entry
    fn create_plain_text_sentry_log(&self, message: &str) -> SentryLogPayload {
        let mut attributes = HashMap::new();

        // Add SDK information
        attributes.insert(
            "sentry.sdk.name".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String("sentry-cli".to_owned()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "sentry.sdk.version".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(env!("CARGO_PKG_VERSION").to_owned()),
                attr_type: "string".to_owned(),
            },
        );

        // Add format information
        attributes.insert(
            "log_format".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String("plain".to_owned()),
                attr_type: "string".to_owned(),
            },
        );

        // Add source information
        attributes.insert(
            "log_source".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String("sentry-cli-tail".to_owned()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "organization".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(self.org.clone()),
                attr_type: "string".to_owned(),
            },
        );
        attributes.insert(
            "project".to_owned(),
            SentryLogAttribute {
                value: serde_json::Value::String(self.project.clone()),
                attr_type: "string".to_owned(),
            },
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        // Generate a random trace_id (32 hex chars)
        let trace_id = format!("{:032x}", rand::random::<u128>());

        SentryLogPayload {
            timestamp,
            trace_id,
            level: "info".to_owned(),
            body: message.to_owned(),
            attributes,
            severity_number: Some(9), // Info level is 9-12, we use 9
        }
    }

    /// Send a raw envelope to Sentry using the updated envelope API
    fn send_raw_envelope(&self, envelope_bytes: Vec<u8>) -> Result<()> {
        self.envelope_api
            .send_raw_envelope(envelope_bytes)
            .context("Failed to send raw log envelope to Sentry")?;
        Ok(())
    }
}

/// Convert our LogLevel to Sentry log level and severity number
/// Returns (level_string, severity_number) as per Sentry logs protocol
fn convert_log_level_to_sentry(level: &Option<LogLevel>) -> (String, u8) {
    match level {
        Some(LogLevel::Debug) => ("debug".to_owned(), 5), // Debug level: 5-8
        Some(LogLevel::Info) => ("info".to_owned(), 9),   // Info level: 9-12
        Some(LogLevel::Warning) => ("warn".to_owned(), 13), // Warn level: 13-16
        Some(LogLevel::Error) => ("error".to_owned(), 17), // Error level: 17-20
        Some(LogLevel::Fatal) => ("fatal".to_owned(), 21), // Fatal level: 21-24
        None => ("info".to_owned(), 9),                   // Default to info for unspecified levels
    }
}

/// Rate limiter for controlling log transmission frequency
pub struct TransmissionRateLimiter {
    max_events_per_minute: u32,
    current_minute: DateTime<Utc>,
    events_this_minute: u32,
}

impl TransmissionRateLimiter {
    /// Create a new rate limiter
    pub fn new(max_events_per_minute: u32) -> Self {
        TransmissionRateLimiter {
            max_events_per_minute,
            current_minute: Utc::now(),
            events_this_minute: 0,
        }
    }

    /// Check if we can send more events, and update counters
    pub fn can_send(&mut self) -> bool {
        let now = Utc::now();

        // Reset counter if we've moved to a new minute
        use chrono::Timelike as _;
        if now.minute() != self.current_minute.minute() || now.hour() != self.current_minute.hour()
        {
            self.current_minute = now;
            self.events_this_minute = 0;
        }

        if self.events_this_minute >= self.max_events_per_minute {
            false
        } else {
            self.events_this_minute += 1;
            true
        }
    }

    /// Get current rate limit status
    pub fn get_status(&self) -> (u32, u32) {
        (self.events_this_minute, self.max_events_per_minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::log_parsing::LogLevel;

    #[test]
    fn test_convert_log_level_to_sentry() {
        assert_eq!(
            convert_log_level_to_sentry(&Some(LogLevel::Debug)),
            ("debug".to_owned(), 5)
        );
        assert_eq!(
            convert_log_level_to_sentry(&Some(LogLevel::Info)),
            ("info".to_owned(), 9)
        );
        assert_eq!(
            convert_log_level_to_sentry(&Some(LogLevel::Warning)),
            ("warn".to_owned(), 13)
        );
        assert_eq!(
            convert_log_level_to_sentry(&Some(LogLevel::Error)),
            ("error".to_owned(), 17)
        );
        assert_eq!(
            convert_log_level_to_sentry(&Some(LogLevel::Fatal)),
            ("fatal".to_owned(), 21)
        );
        assert_eq!(convert_log_level_to_sentry(&None), ("info".to_owned(), 9));
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = TransmissionRateLimiter::new(2);

        assert!(limiter.can_send()); // 1st event
        assert!(limiter.can_send()); // 2nd event
        assert!(!limiter.can_send()); // Should be rate limited

        let (current, max) = limiter.get_status();
        assert_eq!(current, 2);
        assert_eq!(max, 2);
    }
}
