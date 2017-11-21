//! Provides support for sending events to Sentry.
use std::collections::HashMap;
use std::process::Command;

use prelude::*;
use utils::to_timestamp;
use chrono::Utc;
use serde_json::Value;


#[derive(Serialize)]
pub struct Message {
    pub message: String,
    #[serde(skip_serializing_if="Vec::is_empty")]
    pub params: Vec<String>,
}

#[derive(Serialize)]
pub struct Breadcrumb {
    pub timestamp: Option<f64>,
    #[serde(rename="type")]
    pub ty: String,
    pub message: String,
    pub category: String,
}

/// Represents a Sentry event.
#[derive(Serialize)]
pub struct Event {
    pub tags: HashMap<String, String>,
    pub extra: HashMap<String, Value>,
    pub level: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub fingerprint: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none", rename="sentry.interfaces.Message")]
    pub message: Option<Message>,
    pub platform: String,
    pub timestamp: f64,
    #[serde(skip_serializing_if="Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub release: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub dist: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if="HashMap::is_empty")]
    pub user: HashMap<String, String>,
    #[serde(skip_serializing_if="HashMap::is_empty")]
    pub contexts: HashMap<String, HashMap<String, String>>,
    #[serde(skip_serializing_if="Vec::is_empty")]
    pub breadcrumbs: Vec<Breadcrumb>,
}

fn get_server_name() -> Result<String> {
    let p = Command::new("uname").arg("-n").output()?;
    Ok(String::from_utf8(p.stdout)?.trim().to_owned())
}

impl Event {
    pub fn new() -> Event {
        Event {
            tags: HashMap::new(),
            extra: HashMap::new(),
            level: "error".into(),
            fingerprint: None,
            message: None,
            platform: "other".into(),
            timestamp: to_timestamp(Utc::now()),
            server_name: get_server_name().ok(),
            release: None,
            dist: None,
            environment: None,
            user: HashMap::new(),
            contexts: HashMap::new(),
            breadcrumbs: Vec::new(),
        }
    }
}
