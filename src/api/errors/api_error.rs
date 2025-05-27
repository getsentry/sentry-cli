use std::fmt;

#[derive(Debug, thiserror::Error)]
pub struct ApiError {
    inner: ApiErrorKind,
    #[source]
    source: Option<anyhow::Error>,
}

/// Represents API errors.
#[derive(Copy, Clone, Eq, PartialEq, Debug, thiserror::Error)]
pub(in crate::api) enum ApiErrorKind {
    #[error("could not serialize value as JSON")]
    CannotSerializeAsJson,
    #[error("could not serialize envelope")]
    CannotSerializeEnvelope,
    #[error("could not parse JSON response")]
    BadJson,
    #[error("not a JSON response")]
    NotJson,
    #[error("request failed because API URL was incorrectly formatted")]
    BadApiUrl,
    #[error("organization not found")]
    OrganizationNotFound,
    #[error("resource not found")]
    ResourceNotFound,
    #[error("Project not found. Ensure that you configured the correct project and organization.")]
    ProjectNotFound,
    #[error("Release not found. Ensure that you configured the correct release, project, and organization.")]
    ReleaseNotFound,
    #[error("chunk upload endpoint not supported by sentry server")]
    ChunkUploadNotSupported,
    #[error("API request failed")]
    RequestFailed,
    #[error("could not compress data")]
    CompressionFailed,
    #[error("region overrides cannot be applied to absolute urls")]
    InvalidRegionRequest,
    #[error(
        "Auth token is required for this request. Please run `sentry-cli login` and try again!"
    )]
    AuthMissing,
    #[error(
        "DSN missing. Please set the `SENTRY_DSN` environment variable to your project's DSN."
    )]
    DsnMissing,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl ApiError {
    pub(in crate::api) fn with_source<E>(kind: ApiErrorKind, source: E) -> ApiError
    where
        E: Into<anyhow::Error>,
    {
        ApiError {
            inner: kind,
            source: Some(source.into()),
        }
    }

    pub(in crate::api) fn kind(&self) -> ApiErrorKind {
        self.inner
    }

    fn set_source<E: Into<anyhow::Error>>(mut self, source: E) -> ApiError {
        self.source = Some(source.into());
        self
    }
}

impl From<ApiErrorKind> for ApiError {
    fn from(kind: ApiErrorKind) -> ApiError {
        ApiError {
            inner: kind,
            source: None,
        }
    }
}

impl From<curl::Error> for ApiError {
    fn from(err: curl::Error) -> ApiError {
        ApiError::from(ApiErrorKind::RequestFailed).set_source(err)
    }
}

impl From<curl::FormError> for ApiError {
    fn from(err: curl::FormError) -> ApiError {
        ApiError::from(ApiErrorKind::RequestFailed).set_source(err)
    }
}
