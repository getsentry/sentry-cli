use super::{
    errors::{ApiErrorKind, ApiResult},
    Api, Method,
};
use crate::constants::USER_AGENT;
use anyhow::{anyhow, Result};
use sentry::{protocol::EnvelopeItem, types::Dsn, Envelope};
use std::sync::Arc;

pub struct EnvelopesApi {
    api: Arc<Api>,
    dsn: Dsn,
}

impl EnvelopesApi {
    pub fn try_new() -> ApiResult<EnvelopesApi> {
        let api = Api::current();
        match api.config.get_dsn() {
            Ok(dsn) => Ok(EnvelopesApi { api, dsn }),
            Err(_) => Err(ApiErrorKind::DsnMissing.into()),
        }
    }

    pub fn send_item(&self, item: EnvelopeItem) -> Result<()> {
        let mut envelope = Envelope::new();
        envelope.add_item(item);
        self.send_envelope(envelope)
    }

    pub fn send_envelope(&self, envelope: Envelope) -> Result<()> {
        let mut body: Vec<u8> = Vec::new();
        envelope.to_writer(&mut body)?;
        let url = self.dsn.envelope_api_url();
        let auth = self.dsn.to_auth(Some(USER_AGENT));
        let response = self
            .api
            .request(Method::Post, url.as_str(), None)?
            .with_header("X-Sentry-Auth", &auth.to_string())?
            .with_body(body)?
            .send()?;
        match response.ok() {
            true => Ok(()),
            false => Err(anyhow!("Failed to send envelope.")),
        }
    }
}
