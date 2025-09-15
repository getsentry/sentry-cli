use anyhow::{anyhow, Result};
use clap::Args;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

use super::log::Log;
use crate::api::envelopes_api::EnvelopesApi;

#[derive(Args)]
pub(super) struct SendLogsArgs {
    #[arg(long = "level", value_parser = ["trace", "debug", "info", "warn", "error", "fatal"], default_value = "info", help = "Log severity level.")]
    pub(super) level: String,

    #[arg(long = "message", help = "Log message body.")]
    pub(super) message: String,

    #[arg(
        long = "trace-id",
        value_name = "TRACE_ID",
        required = false,
        help = "Optional 32-char hex trace id. If omitted, a random one is generated."
    )]
    pub(super) trace_id: Option<String>,

    #[arg(
        long = "release",
        short = 'r',
        value_name = "RELEASE",
        help = "Optional release identifier. Defaults to auto-detected value."
    )]
    pub(super) release: Option<String>,

    #[arg(
        long = "env",
        short = 'E',
        value_name = "ENVIRONMENT",
        help = "Optional environment name."
    )]
    pub(super) environment: Option<String>,

    #[arg(long = "attr", short = 'a', value_name = "KEY:VALUE", action = clap::ArgAction::Append, help = "Add attributes to the log (key:value pairs). Can be used multiple times.")]
    pub(super) attributes: Vec<String>,
}

#[derive(Clone)]
pub(super) struct LogLevel(pub String);

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" | "debug" | "info" | "warn" | "error" | "fatal" => Ok(LogLevel(s.to_owned())),
            _ => Err(anyhow!(
                "Invalid log level '{}'. Must be one of: trace, debug, info, warn, error, fatal",
                s
            )),
        }
    }
}

impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize)]
pub(super) struct LogItem<'a> {
    pub(super) timestamp: f64,
    #[serde(rename = "trace_id")]
    pub(super) trace_id: &'a str,
    pub(super) level: &'a str,
    #[serde(rename = "body")]
    pub(super) body: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) severity_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) attributes: Option<HashMap<String, AttributeValue>>,
}

#[derive(Serialize)]
pub(super) struct AttributeValue {
    pub(super) value: Value,
    #[serde(rename = "type")]
    pub(super) attr_type: String,
}

impl LogLevel {
    pub(super) fn to_severity_number(&self) -> i32 {
        match self.0.as_str() {
            "trace" => 1,
            "debug" => 5,
            "info" => 9,
            "warn" => 13,
            "error" => 17,
            "fatal" => 21,
            _ => 9,
        }
    }
}

pub(super) fn execute(args: SendLogsArgs) -> Result<()> {
    // Validate trace id if provided
    if let Some(tid) = &args.trace_id {
        let is_valid = tid.len() == 32 && tid.chars().all(|c| c.is_ascii_hexdigit());
        if !is_valid {
            return Err(anyhow!("trace-id must be a 32-character hex string"));
        }
    }

    // Parse attributes from command line
    let mut attr_pairs = Vec::new();
    for attr in &args.attributes {
        let parts: Vec<&str> = attr.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid attribute format '{}'. Expected 'key:value'",
                attr
            ));
        }
        attr_pairs.push((parts[0].to_owned(), parts[1].to_owned()));
    }

    // Build log using the builder pattern
    let mut log = Log::new(args.level.clone(), args.message.clone()).with_attributes(attr_pairs);

    if let Some(trace_id) = args.trace_id {
        log = log.with_trace_id(trace_id);
    }

    if let Some(release) = args.release {
        log = log.with_release(release);
    }

    if let Some(environment) = args.environment {
        log = log.with_environment(environment);
    }

    // Convert to envelope and send
    let envelope = log.into_envelope()?;
    EnvelopesApi::try_new()?.send_envelope(envelope)?;

    println!("Log sent.");
    Ok(())
}
