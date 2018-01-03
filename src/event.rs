//! Provides support for sending events to Sentry.
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use std::process::Command;

#[cfg(not(windows))]
use uname::uname;
use chrono::Utc;
use serde_json::Value;
use username::get_user_name;
use hostname::get_hostname;
use anylog::LogEntry;
use regex::Regex;

use prelude::*;
use constants::{ARCH, PLATFORM};
use utils::{to_timestamp, get_model, get_family, detect_release_name};

lazy_static! {
    static ref COMPONENT_RE: Regex = Regex::new(
        r#"^([^:]+): (.*)$"#).unwrap();
}


#[derive(Serialize)]
pub struct Message {
    pub message: String,
    #[serde(skip_serializing_if="Vec::is_empty")]
    pub params: Vec<String>,
}

#[derive(Serialize)]
pub struct Exception {
    pub values: Vec<SingleException>,
}

#[derive(Serialize, Default)]
pub struct Frame {
    pub filename: String,
    pub abs_path: Option<String>,
    pub function: String,
    pub lineno: Option<u32>,
    pub context_line: Option<String>,
    pub pre_context: Option<Vec<String>>,
    pub post_context: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct Stacktrace {
    pub frames: Vec<Frame>,
}

#[derive(Serialize)]
pub struct SingleException {
    #[serde(rename="type")]
    pub ty: String,
    pub value: String,
    pub stacktrace: Option<Stacktrace>,
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
    pub exception: Option<Exception>,
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
            exception: None,
        }
    }

    pub fn new_prefilled() -> Result<Event> {
        let mut event = Event::new();

        event.extra.insert("environ".into(), Value::Object(env::vars().map(|(k, v)| {
            (k, Value::String(v))
        }).collect()));

        event.user.insert("username".into(), get_user_name().unwrap_or("unknown".into()));
        event.user.insert("ip_address".into(), String::from("{{auto}}"));

        let mut device = HashMap::new();
        if let Some(hostname) = get_hostname() {
            device.insert("name".into(), hostname);
        }
        if let Some(model) = get_model() {
            device.insert("model".into(), model);
        }
        if let Some(family) = get_family() {
            device.insert("family".into(), family);
        }
        device.insert("arch".into(), ARCH.into());
        event.contexts.insert("device".into(), device);

        let mut os = HashMap::new();
        #[cfg(not(windows))] {
            if let Ok(info) = uname() {
                os.insert("name".into(), info.sysname);
                os.insert("kernel_version".into(), info.version);
                os.insert("version".into(), info.release);
            }
        }
        if !os.contains_key("name") {
            os.insert("name".into(), PLATFORM.into());
        }
        event.contexts.insert("os".into(), os);
        Ok(event)
    }

    pub fn detect_release(&mut self) {
        self.release = detect_release_name().ok();
    }

    pub fn attach_logfile(&mut self, logfile: &str, with_component: bool) -> Result<()> {
        let f = fs::File::open(logfile)
            .chain_err(|| "Could not open logfile")?;
        let reader = BufReader::new(f);
        for line in reader.lines() {
            let line = line?;
            let rec = LogEntry::parse(line.as_bytes());
            let component;
            let message;

            if_chain! {
                if with_component;
                if let Some(caps) = COMPONENT_RE.captures(&rec.message());
                then {
                    component = caps.get(1).map(|x| x.as_str().to_string()).unwrap();
                    message = caps.get(2).map(|x| x.as_str().to_string()).unwrap();
                } else {
                    component = "log".to_string();
                    message = rec.message().to_string();
                }
            }

            self.breadcrumbs.push(Breadcrumb {
                timestamp: rec.utc_timestamp().map(|x| x.timestamp() as f64),
                message: message,
                ty: "default".to_string(),
                category: component.to_string(),
            })
        }

        if self.breadcrumbs.len() > 100 {
            let skip = self.breadcrumbs.len() - 100;
            self.breadcrumbs.drain(..skip);
        }

        Ok(())
    }
}
