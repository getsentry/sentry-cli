use super::send::{AttributeValue, LogItem, LogLevel};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::event::get_sdk_info;
use crate::utils::releases::detect_release_name;

/// Log entry struct.
pub struct Log {
    level: LogLevel,
    message: String,
    trace_id: Option<String>,
    release: Option<String>,
    environment: Option<String>,
    attributes: HashMap<String, AttributeValue>,
}

impl Log {
    /// Create a new log entry with the specified level and message.
    pub fn new(level: String, message: String) -> Self {
        Self {
            level: LogLevel(level),
            message,
            trace_id: None,
            release: None,
            environment: None,
            attributes: HashMap::new(),
        }
    }

    /// Set the trace ID for this log entry.
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Set the release for this log entry.
    pub fn with_release(mut self, release: String) -> Self {
        self.release = Some(release);
        self
    }

    /// Set the environment for this log entry.
    pub fn with_environment(mut self, environment: String) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Add multiple attributes from key-value pairs.
    pub fn with_attributes(mut self, attrs: Vec<(String, String)>) -> Self {
        for (key, value_str) in attrs {
            let (value, attr_type) = parse_attribute_value(&value_str);
            self.attributes
                .insert(key, AttributeValue { value, attr_type });
        }
        self
    }

    /// Convert this log entry to a Sentry envelope.
    pub fn into_envelope(mut self) -> Result<sentry::Envelope> {
        // Generate trace ID if not provided
        let trace_id = self.trace_id.take().unwrap_or_else(generate_trace_id);

        // Add SDK attributes
        let mut attributes = self.attributes;
        add_sdk_attributes(&mut attributes);

        // Add release if provided or auto-detected
        let release = self.release.or_else(|| detect_release_name().ok());
        if let Some(rel) = &release {
            attributes.insert(
                "sentry.release".to_owned(),
                AttributeValue {
                    value: Value::String(rel.clone()),
                    attr_type: "string".to_owned(),
                },
            );
        }

        // Add environment if provided
        if let Some(env) = &self.environment {
            attributes.insert(
                "sentry.environment".to_owned(),
                AttributeValue {
                    value: Value::String(env.clone()),
                    attr_type: "string".to_owned(),
                },
            );
        }

        let log_item = LogItem {
            timestamp: now_timestamp_seconds(),
            trace_id: &trace_id,
            level: self.level.as_ref(),
            body: &self.message,
            severity_number: Some(self.level.to_severity_number()),
            attributes: if attributes.is_empty() {
                None
            } else {
                Some(attributes)
            },
        };

        let payload = json!({
            "items": [log_item]
        });

        let payload_bytes = serde_json::to_vec(&payload)?;
        let header = json!({
            "type": "log",
            "item_count": 1,
            "content_type": "application/vnd.sentry.items.log+json",
            "length": payload_bytes.len()
        });
        let header_bytes = serde_json::to_vec(&header)?;

        // Construct raw envelope: metadata line (empty for logs), then header, then payload
        let mut buf = Vec::new();
        // Empty envelope metadata with no event_id
        buf.extend_from_slice(b"{}\n");
        buf.extend_from_slice(&header_bytes);
        buf.push(b'\n');
        buf.extend_from_slice(&payload_bytes);

        Ok(sentry::Envelope::from_bytes_raw(buf)?)
    }
}

fn parse_attribute_value(value_str: &str) -> (Value, String) {
    // Try to parse as different types
    let (value, attr_type) = if let Ok(b) = value_str.parse::<bool>() {
        (Value::Bool(b), "boolean".to_owned())
    } else if let Ok(i) = value_str.parse::<i64>() {
        (
            Value::Number(serde_json::Number::from(i)),
            "integer".to_owned(),
        )
    } else if let Ok(f) = value_str.parse::<f64>() {
        (
            Value::Number(serde_json::Number::from_f64(f).expect("Failed to parse float")),
            "double".to_owned(),
        )
    } else {
        (Value::String(value_str.to_owned()), "string".to_owned())
    };

    (value, attr_type)
}

fn add_sdk_attributes(attributes: &mut HashMap<String, AttributeValue>) {
    let sdk_info = get_sdk_info();

    attributes.insert(
        "sentry.sdk.name".to_owned(),
        AttributeValue {
            value: Value::String(sdk_info.name.to_owned()),
            attr_type: "string".to_owned(),
        },
    );

    attributes.insert(
        "sentry.sdk.version".to_owned(),
        AttributeValue {
            value: Value::String(sdk_info.version.to_owned()),
            attr_type: "string".to_owned(),
        },
    );
}

fn now_timestamp_seconds() -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    now.as_secs() as f64 + (now.subsec_nanos() as f64) / 1_000_000_000.0
}

fn generate_trace_id() -> String {
    // Generate 16 random bytes, hex-encoded to 32 chars. UUID v4 is 16 random bytes.
    let uuid = uuid::Uuid::new_v4();
    data_encoding::HEXLOWER.encode(uuid.as_bytes())
}
