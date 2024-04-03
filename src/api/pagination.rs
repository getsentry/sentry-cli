use crate::utils::http;

#[derive(Debug, Clone)]
struct Link<'p> {
    results: bool,
    cursor: &'p str,
}

#[derive(Debug, Clone)]
pub(super) struct Pagination<'p> {
    next: Option<Link<'p>>,
}

impl<'p> Pagination<'p> {
    pub fn next_cursor(&self) -> Option<&str> {
        self.next
            .as_ref()
            .and_then(|x| if x.results { Some(x.cursor) } else { None })
    }
}

impl<'p> From<&'p str> for Pagination<'p> {
    fn from(value: &'p str) -> Self {
        http::parse_link_header(value)
            .iter()
            .rev() // Reversing is necessary for backwards compatibility with a previous implementation
            .find(|item| item.get("rel") == Some(&"next"))
            .map_or(Pagination { next: None }, |item| Pagination {
                next: Some(Link {
                    results: item.get("results") == Some(&"true"),
                    cursor: item.get("cursor").unwrap_or(&""),
                }),
            })
    }
}
