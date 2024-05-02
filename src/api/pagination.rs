use std::str::FromStr;

use crate::utils::http;

#[derive(Debug, Clone)]
pub struct Link {
    results: bool,
    cursor: String,
}

#[derive(Debug, Default, Clone)]
pub struct Pagination {
    next: Option<Link>,
}

impl Pagination {
    pub fn into_next_cursor(self) -> Option<String> {
        self.next
            .and_then(|x| if x.results { Some(x.cursor) } else { None })
    }
}

impl FromStr for Pagination {
    type Err = ();

    fn from_str(s: &str) -> Result<Pagination, ()> {
        let mut rv = Pagination::default();
        for item in http::parse_link_header(s) {
            let target = match item.get("rel") {
                Some(&"next") => &mut rv.next,
                _ => continue,
            };

            *target = Some(Link {
                results: item.get("results") == Some(&"true"),
                cursor: (*item.get("cursor").unwrap_or(&"")).to_string(),
            });
        }

        Ok(rv)
    }
}
