use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, warn};
use sentry::protocol::{Event, Level, LogEntry as SentryLogEntry};
use sentry::types::Uuid;
use sentry::{apply_defaults, Client, ClientOptions};
use std::borrow::Cow;

use std::time::SystemTime;

use crate::api::envelopes_api::EnvelopesApi;
use crate::constants::USER_AGENT;
use crate::utils::event::get_sdk_info;
use crate::utils::log_parsing::{LogEntry, LogLevel};

/// Converts our parsed log entries into Sentry events and sends them
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

    /// Send a batch of log entries to Sentry
    pub fn send_log_batch(&self, log_entries: Vec<String>) -> Result<Vec<Uuid>> {
        let mut event_ids = Vec::new();

        for log_entry_json in log_entries {
            match self.process_log_entry(&log_entry_json) {
                Ok(event_id) => {
                    event_ids.push(event_id);
                    debug!("Sent log entry to Sentry: {}", event_id);
                }
                Err(e) => {
                    warn!("Failed to send log entry: {}", e);
                    // Continue processing other entries even if one fails
                }
            }
        }

        debug!("Sent {} log entries to Sentry", event_ids.len());
        Ok(event_ids)
    }

    /// Process a single log entry JSON string and send it to Sentry
    fn process_log_entry(&self, log_entry_json: &str) -> Result<Uuid> {
        // First, try to deserialize as our LogEntry type
        match serde_json::from_str::<LogEntry>(log_entry_json) {
            Ok(log_entry) => self.send_structured_log(&log_entry),
            Err(_) => {
                // Fall back to treating it as plain text
                self.send_plain_text_log(log_entry_json)
            }
        }
    }

    /// Send a structured log entry to Sentry
    fn send_structured_log(&self, log_entry: &LogEntry) -> Result<Uuid> {
        let mut event = Event {
            sdk: Some(get_sdk_info()),
            level: convert_log_level(&log_entry.level),
            platform: Cow::from("other"),
            logentry: Some(SentryLogEntry {
                message: log_entry.message.clone(),
                params: Vec::new(),
            }),
            ..Event::default()
        };

        // Set timestamp if available
        if let Some(timestamp) = log_entry.timestamp {
            event.timestamp = SystemTime::from(timestamp);
        }

        // Add structured fields as extra data
        for (key, value) in &log_entry.fields {
            event.extra.insert(
                key.clone().into(), 
                serde_json::Value::String(value.clone())
            );
        }

        // Add format information
        event.extra.insert(
            "log_format".into(),
            serde_json::Value::String(log_entry.format.name().to_string())
        );

        // Add source information
        event.extra.insert(
            "log_source".into(),
            serde_json::Value::String("sentry-cli-tail".to_string())
        );
        event.extra.insert(
            "organization".into(),
            serde_json::Value::String(self.org.clone())
        );
        event.extra.insert(
            "project".into(),
            serde_json::Value::String(self.project.clone())
        );

        // Set tags for better filtering in Sentry
        event.tags.insert("log_format".into(), log_entry.format.name().into());
        if let Some(level) = &log_entry.level {
            event.tags.insert("original_level".into(), level.to_sentry_level().into());
        }

        // Add HTTP-specific tags if available
        if let Some(status) = log_entry.fields.get("status") {
            event.tags.insert("http_status".into(), status.clone().into());
        }
        if let Some(remote_addr) = log_entry.fields.get("remote_addr").or_else(|| log_entry.fields.get("remote_host")) {
            event.tags.insert("client_ip".into(), remote_addr.clone().into());
        }

        self.send_event(event)
    }

    /// Send plain text as a log entry to Sentry
    fn send_plain_text_log(&self, message: &str) -> Result<Uuid> {
        let event = Event {
            sdk: Some(get_sdk_info()),
            level: Level::Info,
            platform: Cow::from("other"),
            logentry: Some(SentryLogEntry {
                message: message.to_string(),
                params: Vec::new(),
            }),
            extra: [
                ("log_source".into(), serde_json::Value::String("sentry-cli-tail".to_string())),
                ("log_format".into(), serde_json::Value::String("plain".to_string())),
                ("organization".into(), serde_json::Value::String(self.org.clone())),
                ("project".into(), serde_json::Value::String(self.project.clone())),
            ].into(),
            tags: [
                ("log_format".into(), "plain".into()),
                ("original_level".into(), "info".into()),
            ].into(),
            ..Event::default()
        };

        self.send_event(event)
    }

    /// Send a Sentry event using the envelope API
    fn send_event(&self, event: Event<'static>) -> Result<Uuid> {
        let client = Client::from_config(apply_defaults(ClientOptions {
            user_agent: USER_AGENT.into(),
            ..Default::default()
        }));

        let event = client
            .prepare_event(event, None)
            .context("Event dropped during preparation")?;

        let event_id = event.event_id;
        
        self.envelope_api
            .send_envelope(event)
            .context("Failed to send envelope to Sentry")?;

        Ok(event_id)
    }
}

/// Convert our LogLevel to Sentry Level
fn convert_log_level(level: &Option<LogLevel>) -> Level {
    match level {
        Some(LogLevel::Debug) => Level::Debug,
        Some(LogLevel::Info) => Level::Info,
        Some(LogLevel::Warning) => Level::Warning,
        Some(LogLevel::Error) => Level::Error,
        Some(LogLevel::Fatal) => Level::Fatal,
        None => Level::Info, // Default to info for unspecified levels
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
        use chrono::Timelike;
        if now.minute() != self.current_minute.minute() || now.hour() != self.current_minute.hour() {
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
    fn test_convert_log_level() {
        assert_eq!(convert_log_level(&Some(LogLevel::Debug)), Level::Debug);
        assert_eq!(convert_log_level(&Some(LogLevel::Info)), Level::Info);
        assert_eq!(convert_log_level(&Some(LogLevel::Warning)), Level::Warning);
        assert_eq!(convert_log_level(&Some(LogLevel::Error)), Level::Error);
        assert_eq!(convert_log_level(&Some(LogLevel::Fatal)), Level::Fatal);
        assert_eq!(convert_log_level(&None), Level::Info);
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
