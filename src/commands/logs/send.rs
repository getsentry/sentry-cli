use anyhow::{anyhow, Result};
use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::envelopes_api::EnvelopesApi;
use crate::utils::event::get_sdk_info;
use crate::utils::releases::detect_release_name;

#[derive(Args)]
pub(super) struct SendLogsArgs {
    #[arg(long = "level", value_parser = ["trace", "debug", "info", "warn", "error", "fatal"], default_value = "info", help = "Log severity level.")]
    level: String,

    #[arg(long = "message", help = "Log message body.")]
    message: String,

    #[arg(
        long = "trace-id",
        value_name = "TRACE_ID",
        required = false,
        help = "Optional 32-char hex trace id. If omitted, a random one is generated."
    )]
    trace_id: Option<String>,

    #[arg(
        long = "release",
        short = 'r',
        value_name = "RELEASE",
        help = "Optional release identifier. Defaults to auto-detected value."
    )]
    release: Option<String>,

    #[arg(
        long = "env",
        short = 'E',
        value_name = "ENVIRONMENT",
        help = "Optional environment name."
    )]
    environment: Option<String>,

    #[arg(long = "attr", short = 'a', value_name = "KEY:VALUE", action = clap::ArgAction::Append, help = "Add attributes to the log (key:value pairs). Can be used multiple times.")]
    attributes: Vec<String>,
}

#[derive(Serialize)]
struct LogItem<'a> {
    timestamp: f64,
    #[serde(rename = "trace_id")]
    trace_id: &'a str,
    level: &'a str,
    #[serde(rename = "body")]
    body: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attributes: Option<HashMap<String, AttributeValue>>,
}

#[derive(Serialize)]
struct AttributeValue {
    value: Value,
    #[serde(rename = "type")]
    attr_type: String,
}

fn level_to_severity_number(level: &str) -> i32 {
    match level {
        "trace" => 1,
        "debug" => 5,
        "info" => 9,
        "warn" => 13,
        "error" => 17,
        "fatal" => 21,
        _ => 9,
    }
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

fn parse_attributes(attrs: &[String]) -> Result<HashMap<String, AttributeValue>> {
    let mut attributes = HashMap::new();

    for attr in attrs {
        let parts: Vec<&str> = attr.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid attribute format '{}'. Expected 'key:value'",
                attr
            ));
        }

        let key = parts[0].to_owned();
        let value_str = parts[1];

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

        attributes.insert(key, AttributeValue { value, attr_type });
    }

    Ok(attributes)
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

pub(super) fn execute(args: SendLogsArgs) -> Result<()> {
    // Note: The org and project values are not needed for sending logs,
    // as the EnvelopesApi uses the DSN from config which already contains this information.

    // Validate trace id or generate a new one
    let trace_id_owned;
    let trace_id = if let Some(tid) = &args.trace_id {
        let is_valid = tid.len() == 32 && tid.chars().all(|c| c.is_ascii_hexdigit());
        if !is_valid {
            return Err(anyhow!("trace-id must be a 32-character hex string"));
        }
        tid.as_str()
    } else {
        trace_id_owned = generate_trace_id();
        &trace_id_owned
    };

    let severity_number = level_to_severity_number(&args.level);

    let mut attributes = parse_attributes(&args.attributes)?;

    add_sdk_attributes(&mut attributes);

    let release = args.release.clone().or_else(|| detect_release_name().ok());
    if let Some(rel) = &release {
        attributes.insert(
            "sentry.release".to_owned(),
            AttributeValue {
                value: Value::String(rel.clone()),
                attr_type: "string".to_owned(),
            },
        );
    }

    if let Some(env) = &args.environment {
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
        trace_id,
        level: &args.level,
        body: &args.message,
        severity_number: Some(severity_number),
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

    let envelope = sentry::Envelope::from_bytes_raw(buf)?;
    EnvelopesApi::try_new()?.send_envelope(envelope)?;

    println!("Log sent.");
    Ok(())
}
