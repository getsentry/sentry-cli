mod api_error;
mod sentry_error;

pub(super) use api_error::{ApiError, ApiErrorKind};
pub(super) use sentry_error::SentryError;

use crate::api::ApiResponse;

#[derive(Clone, Debug, thiserror::Error)]
#[error("project was renamed to '{0}'\nPlease use this slug in your .sentryclirc file, sentry.properties file or in the CLI --project parameter")]
pub(super) struct ProjectRenamedError(pub(super) String);

/// Shortcut alias for results of this module.
pub(super) type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub(super) enum RetryError {
    #[error("request failed with retryable status code {}", body.status)]
    Status { body: ApiResponse },
    #[error("request failed with retryable error: {source}")]
    ApiError {
        #[from]
        source: ApiError,
    },
}
