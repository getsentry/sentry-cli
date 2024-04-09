use std::fmt;

#[derive(Debug, thiserror::Error)]
pub(in crate::api) struct SentryError {
    pub(in crate::api) status: u32,
    pub(in crate::api) detail: Option<String>,
    pub(in crate::api) extra: Option<serde_json::Value>,
}

impl fmt::Display for SentryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = self.detail.as_deref().unwrap_or("");
        write!(
            f,
            "sentry reported an error: {} (http status: {})",
            if detail.is_empty() {
                match self.status {
                    400 => "bad request",
                    401 => "unauthorized",
                    404 => "not found",
                    500 => "internal server error",
                    502 => "bad gateway",
                    504 => "gateway timeout",
                    _ => "unknown error",
                }
            } else {
                detail
            },
            self.status
        )?;
        if let Some(ref extra) = self.extra {
            write!(f, "\n  {extra:?}")?;
        }
        Ok(())
    }
}
