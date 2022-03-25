use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use log::Log;
use sentry::integrations::anyhow::AnyhowHubExt;
use sentry::{release_name, Client, ClientOptions, Hub};

use crate::config::Config;
use crate::constants::USER_AGENT;

pub fn setup(log: Box<dyn Log>) {
    bind_configured_client(None);
    log::set_boxed_logger(log).unwrap();
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
    Hub::with_active(|hub| hub.capture_anyhow(&err));
    flush_events();
}

pub fn flush_events() {
    if let Some(client) = Hub::with(|hub| hub.client()) {
        client.close(Some(Duration::from_secs(2)));
    }
}
