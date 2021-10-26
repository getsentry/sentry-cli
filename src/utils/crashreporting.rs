use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use failure::Error;
use log::Log;
use sentry::{release_name, Client, ClientOptions, Hub};

use crate::config::Config;
use crate::constants::USER_AGENT;

pub fn setup(_log: Box<dyn Log>) {
    bind_configured_client(None);
}

pub fn bind_configured_client(cfg: Option<&Config>) {
    Hub::with(|hub| {
        let dsn = cfg.and_then(Config::internal_sentry_dsn);
        let client = match dsn {
            Some(dsn) => Client::from_config((
                dsn,
                ClientOptions {
                    release: release_name!(),
                    user_agent: Cow::Borrowed(USER_AGENT),
                    ..Default::default()
                },
            )),
            None => Client::from_config(()),
        };

        hub.bind_client(Some(Arc::new(client)))
    });
}

pub fn try_report_to_sentry(err: Error) {
    // TODO: Migrate from `failure` to `anyhow` crate, as `sentry` dropped support for `failure` in version 0.22
    // and use sentry::integrations::anyhow to capture more details about the error.
    Hub::with_active(|hub| hub.capture_error(&err.compat()));
    flush_events();
}

pub fn flush_events() {
    if let Some(client) = Hub::with(|hub| hub.client()) {
        client.close(Some(Duration::from_secs(2)));
    }
}
