use std::time::SystemTime;
use std::collections::HashMap;
use std::process::Command;

use CliResult;
use utils::to_timestamp;


#[derive(Serialize)]
pub struct Device {
    name: String,
    version: String,
    #[serde(skip_serializing_if="Option::is_none")]
    build: Option<String>,
}

impl Device {
    pub fn current() -> CliResult<Device> {
        let p = Command::new("uname").arg("-sr").output()?;
        let output = String::from_utf8(p.stdout)?;
        let mut iter = output.trim().splitn(2, ' ');
        Ok(Device {
            name: iter.next().unwrap_or("Unknown").into(),
            version: iter.next().unwrap_or("?").into(),
            build: None,
        })
    }
}


#[derive(Serialize)]
pub struct Event {
    pub tags: HashMap<String, String>,
    pub extra: HashMap<String, String>,
    pub level: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub message: Option<String>,
    pub platform: String,
    pub timestamp: f64,
    #[serde(skip_serializing_if="Option::is_none")]
    pub device: Option<Device>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub release: Option<String>,
}

fn get_server_name() -> CliResult<String> {
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
            timestamp: to_timestamp(SystemTime::now()),
            device: Device::current().ok(),
            server_name: get_server_name().ok(),
            release: None,
        }
    }
}
