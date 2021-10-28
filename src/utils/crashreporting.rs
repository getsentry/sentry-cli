use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use failure::Error;
use log::Log;
use sentry::{release_name, Client, ClientOptions, Hub};
use sentry_types::Dsn;

use crate::config::Config;
use crate::constants::USER_AGENT;

pub fn setup(_log: Box<dyn Log>) {
    bind_configured_client(None);
}

pub fn bind_configured_client(cfg: Option<&Config>) {
    Hub::with(|hub| {
        let dsn: Option<Dsn> = cfg.and_then(Config::internal_sentry_dsn);
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

pub fn try_report_to_sentry(_err: &Error) {
    // TODO: Migrate from `failure` to `anyhow` crate and use `anyhow` feature for crash reporting once we decide to bring it back.
    // capture_error(err);
    // flush_events();
}

pub fn flush_events() {
    if let Some(client) = Hub::with(|hub| hub.client()) {
        client.close(Some(Duration::from_secs(2)));
    }
}
