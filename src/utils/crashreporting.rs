use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use ::failure::Error;
use ::log::Log;
use sentry::integrations::{failure, log, panic};
use sentry::{Client, ClientOptions, Hub};

use crate::config::Config;
use crate::constants::USER_AGENT;

pub fn setup(log: Box<Log>) {
    log::init(Some(log), Default::default());
    panic::register_panic_handler();
    bind_configured_client(None);
}

pub fn bind_configured_client(cfg: Option<&Config>) {
    Hub::with(|hub| {
        let dsn = cfg.and_then(|config| config.internal_sentry_dsn());
        let client = match dsn {
            Some(dsn) => Client::from_config((
                dsn,
                ClientOptions {
                    release: sentry_crate_release!(),
                    user_agent: Cow::Borrowed(USER_AGENT),
                    ..Default::default()
                },
            )),
            None => Client::from_config(()),
        };

        hub.bind_client(Some(Arc::new(client)))
    });
}

pub fn try_report_to_sentry(err: &Error) {
    failure::capture_error(err);
    flush_events();
}

pub fn flush_events() {
    if let Some(client) = Hub::with(|hub| hub.client()) {
        client.close(Some(Duration::from_secs(2)));
    }
}
