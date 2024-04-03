use crate::utils::http;

pub(super) fn next_cursor(value: &str) -> Option<&str> {
    http::parse_link_header(value)
        .iter()
        .rev() // Reversing is necessary for backwards compatibility with a previous implementation
        .find(|item| item.get("rel") == Some(&"next"))
        .and_then::<&str, _>(|item| {
            if item.get("results") == Some(&"true") {
                Some(item.get("cursor").unwrap_or(&""))
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_empty_string() {
        let result = next_cursor("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_pagination_with_next() {
        let result = next_cursor(
            "<https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:-1:1>; \
            rel=\"previous\"; results=\"false\"; cursor=\"100:-1:1\", \
            <https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:1:0>; \
            rel=\"next\"; results=\"true\"; cursor=\"100:1:0\"",
        );
        assert_eq!(result, Some("100:1:0"));
    }

    #[test]
    fn test_pagination_without_next() {
        let result = next_cursor(
            "<https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:-1:1>; \
            rel=\"previous\"; results=\"false\"; cursor=\"100:-1:1\"",
        );
        assert_eq!(result, None);
    }
}
