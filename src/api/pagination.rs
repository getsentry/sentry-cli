use crate::utils::http;

#[derive(Debug, Clone)]
struct Link<'p> {
    results: bool,
    cursor: &'p str,
}

#[derive(Debug, Default, Clone)]
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
        let mut rv = Pagination::default();
        for item in http::parse_link_header(value) {
            let target = match item.get("rel") {
                Some(&"next") => &mut rv.next,
                _ => continue,
            };

            *target = Some(Link {
                results: item.get("results") == Some(&"true"),
                cursor: item.get("cursor").unwrap_or(&""),
            });
        }

        rv
    }
}
