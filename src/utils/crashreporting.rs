use std::borrow::Cow;
use std::sync::Arc;

use sentry::{self, Client, ClientOptions};
use sentry::integrations::{failure, log, panic};
use log::Log;
use failure::Error;

use config::Config;
use constants::USER_AGENT;

pub fn setup(log: Box<Log>) {
    log::init(Some(log), Default::default());
    panic::register_panic_handler();
    bind_configured_client();
}

pub fn bind_configured_client() {
    Config::get_current_opt()
        .and_then(|config| config.internal_sentry_dsn())
        .and_then(|dsn| {
            Client::from_config((
                dsn,
                ClientOptions {
                    release: sentry_crate_release!(),
                    user_agent: Cow::Borrowed(USER_AGENT),
                    ..Default::default()
                },
            ))
        })
        .map(Arc::new)
        .map(sentry::bind_client);
}

pub fn try_report_to_sentry(err: &Error) {
    failure::capture_error(err);
}
