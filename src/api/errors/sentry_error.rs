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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn error_from_status_and_detail_helper(status: u32, detail: Option<&str>) -> SentryError {
        SentryError {
            status,
            detail: detail.map(|s| s.to_string()),
            extra: None,
        }
    }

    #[rstest]
    #[case(400, "sentry reported an error: bad request (http status: 400)")]
    #[case(401, "sentry reported an error: unauthorized (http status: 401)")]
    #[case(404, "sentry reported an error: not found (http status: 404)")]
    #[case(
        500,
        "sentry reported an error: internal server error (http status: 500)"
    )]
    #[case(502, "sentry reported an error: bad gateway (http status: 502)")]
    #[case(504, "sentry reported an error: gateway timeout (http status: 504)")]
    #[case(600, "sentry reported an error: unknown error (http status: 600)")]
    fn test_display_no_detail_no_extra(
        #[case] status: u32,
        #[case] expected: &str,
        #[values(None, Some(""))] detail: Option<&str>,
    ) {
        let error = error_from_status_and_detail_helper(status, detail);
        assert_eq!(format!("{}", error), expected);
    }

    #[test]
    fn test_display_with_detail_no_extra() {
        let error = error_from_status_and_detail_helper(400, Some("detail"));
        assert_eq!(
            format!("{}", error),
            "sentry reported an error: detail (http status: 400)"
        );
    }

    #[test]
    fn test_display_with_detail_and_extra() {
        let error = SentryError {
            status: 400,
            detail: Some("detail".to_string()),
            extra: Some(serde_json::json!("extra info")),
        };
        assert_eq!(
            format!("{}", error),
            "sentry reported an error: detail (http status: 400)\n  String(\"extra info\")"
        );
    }

    #[test]
    fn test_display_no_detail_with_extra() {
        let error = SentryError {
            status: 400,
            detail: None,
            extra: Some(serde_json::json!("extra info")),
        };
        assert_eq!(
            format!("{}", error),
            "sentry reported an error: bad request (http status: 400)\n  String(\"extra info\")"
        );
    }
}
