use super::{
    errors::{ApiErrorKind, ApiResult},
    Api, ApiResponse, Method,
};
use crate::{api::errors::ApiError, constants::USER_AGENT};
use log::debug;
use sentry::{protocol::EnvelopeItem, types::Dsn, Envelope};
use std::sync::Arc;

pub struct EnvelopesApi {
    api: Arc<Api>,
    dsn: Dsn,
}

impl EnvelopesApi {
    pub fn try_new() -> ApiResult<EnvelopesApi> {
        let api = Api::current();
        api.config
            .get_dsn()
            .map(|dsn| EnvelopesApi { api, dsn })
            .map_err(|_| ApiErrorKind::DsnMissing.into())
    }

    pub fn send_item(&self, item: EnvelopeItem) -> ApiResult<ApiResponse> {
        let mut envelope = Envelope::new();
        envelope.add_item(item);
        self.send_envelope(envelope)
    }

    pub fn send_envelope(&self, envelope: Envelope) -> ApiResult<ApiResponse> {
        let mut body = vec![];
        envelope
            .to_writer(&mut body)
            .map_err(|e| ApiError::with_source(ApiErrorKind::CannotSerializeEnvelope, e))?;
        debug!("Sending envelope:\n{}", String::from_utf8_lossy(&body));
        let url = self.dsn.envelope_api_url();
        let auth = self.dsn.to_auth(Some(USER_AGENT));
        self.api
            .request(Method::Post, url.as_str(), None)?
            .with_header("X-Sentry-Auth", &auth.to_string())?
            .with_body(body)?
            .send()?
            .into_result()
    }
}
