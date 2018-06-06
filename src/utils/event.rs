use std::borrow::Cow;
use std::fs;
use std::io::{BufRead, BufReader};
use std::time::Duration;

use anylog;
use chrono::Utc;
use failure::{Error, ResultExt};
use regex::Regex;
use sentry::protocol::{Breadcrumb, ClientSdkInfo, Event};
use sentry::{Client, ClientOptions, Dsn};

use constants::USER_AGENT;

lazy_static! {
    static ref COMPONENT_RE: Regex = Regex::new(r#"^([^:]+): (.*)$"#).unwrap();
}

/// Attaches all logs from a logfile as breadcrumbs to the given event.
pub fn attach_logfile(event: &mut Event, logfile: &str, with_component: bool) -> Result<(), Error> {
    let f = fs::File::open(logfile).context("Could not open logfile")?;

    // sentry currently requires timestamps for breadcrumbs at all times.
    // Because we might not be able to parse a timestamp from the log file
    // we fall back to either the modified time of the file or if that does
    // not work we use the current timestamp.
    let fallback_timestamp = fs::metadata(logfile)
        .context("Could not get metadata for logfile")?
        .modified()
        .map(|ts| ts.into())
        .unwrap_or_else(|_| Utc::now());

    let reader = BufReader::new(f);
    for line in reader.lines() {
        let line = line?;
        let rec = anylog::LogEntry::parse(line.as_bytes());
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

        event.breadcrumbs.push(Breadcrumb {
            timestamp: rec.utc_timestamp().unwrap_or(fallback_timestamp),
            message: Some(message),
            category: Some(component.to_string()),
            ..Default::default()
        })
    }

    if event.breadcrumbs.len() > 100 {
        let skip = event.breadcrumbs.len() - 100;
        event.breadcrumbs.drain(..skip);
    }

    Ok(())
}

/// Returns SDK information for sentry-cli.
pub fn get_sdk_info() -> Cow<'static, ClientSdkInfo> {
    Cow::Owned(ClientSdkInfo {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        integrations: Vec::new(),
    })
}

/// Executes the callback with an isolate sentry client on an empty isolate scope.
///
/// Use the client's API to capture exceptions or manual events. The client will automatically drop
/// after the callback has finished and drain its queue with a timeout of 2 seconds.
///
/// You can optionally return a value from the callback which will also be the return value of this
/// function if draining the queue is successful. On error, the return value will be None.
pub fn with_sentry_client<F, R>(dsn: Dsn, callback: F) -> Option<R>
where
    F: FnOnce(&Client) -> R,
{
    let client = Client::with_dsn_and_options(dsn, ClientOptions {
        user_agent: USER_AGENT.into(),
        ..Default::default()
    });

    let rv = callback(&client);
    if client.drain_events(Some(Duration::from_secs(2))) {
        Some(rv)
    } else {
        None
    }
}
