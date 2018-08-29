use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use failure::Error;
use log::Log;
use sentry::integrations::{failure, log, panic};
use sentry::{Client, ClientOptions, Hub};

use config::Config;
use constants::USER_AGENT;

pub fn setup(log: Box<Log>) {
    log::init(Some(log), Default::default());
    panic::register_panic_handler();
    bind_configured_client(None);
}

pub fn bind_configured_client(cfg: Option<&Config>) {
    let dsn = if cfg.is_some() {
        cfg.and_then(|config| config.internal_sentry_dsn())
    } else {
        None
    };

    Hub::with(|hub| {
        hub.bind_client(Some(Arc::new(
            dsn.and_then(|dsn| {
                Client::from_config((
                    dsn,
                    ClientOptions {
                        release: sentry_crate_release!(),
                        user_agent: Cow::Borrowed(USER_AGENT),
                        ..Default::default()
                    },
                ))
            }).unwrap_or_else(Client::disabled),
        )))
    });
}

pub fn try_report_to_sentry(err: &Error) {
    failure::capture_error(err);
    flush_events();
}

pub fn flush_events() {
    Hub::with(|hub| hub.drain_events(Some(Duration::from_secs(2))));
}
