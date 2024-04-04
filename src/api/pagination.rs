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
