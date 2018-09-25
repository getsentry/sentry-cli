use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    static ref LINK_TOKEN_RE: Regex = Regex::new(
        r#"(?x)
        (?:
            <(?P<link>[^>]+)>
        ) | (?:
            (?P<key>[a-z]+)
               \s*=\s*
            (?:
                "(?P<qvalue>[^"]+)" |
                (?P<value>[^\s,.]+)
            )
        ) | (?:
            \s*
                (?:
                    (?P<comma>,) |
                    (?P<semi>;)
                )
            \s*
        )
    "#
    ).unwrap();
}

/// Parses a link header into a vector of hash maps.
///
/// The implied `link` tag is stored as `_link`.
pub fn parse_link_header<'a>(s: &'a str) -> Vec<HashMap<&'a str, &'a str>> {
    let mut rv = vec![];
    let mut item = HashMap::new();

    for caps in LINK_TOKEN_RE.captures_iter(s) {
        if let Some(link) = caps.name("link") {
            item.insert("_link", link.as_str());
        } else if let Some(key) = caps.name("key") {
            item.insert(
                key.as_str(),
                caps.name("qvalue")
                    .unwrap_or_else(|| caps.name("value").unwrap())
                    .as_str(),
            );
        } else if caps.name("comma").is_some() {
            rv.push(item);
            item = HashMap::new();
        }
    }

    if !item.is_empty() {
        rv.push(item);
    }

    rv
}

#[test]
fn test_parse_link_header() {
    let rv = parse_link_header("<https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:-1:1>; rel=\"previous\"; results=\"false\"; cursor=\"100:-1:1\", <https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:1:0>; rel=\"next\"; results=\"true\"; cursor=\"100:1:0\"");
    assert_eq!(rv.len(), 2);

    let a = &rv[0];
    let b = &rv[1];

    assert_eq!(
        a.get("_link").unwrap(),
        &"https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:-1:1"
    );
    assert_eq!(a.get("cursor").unwrap(), &"100:-1:1");
    assert_eq!(a.get("rel").unwrap(), &"previous");
    assert_eq!(a.get("results").unwrap(), &"false");

    assert_eq!(
        b.get("_link").unwrap(),
        &"https://sentry.io/api/0/organizations/sentry/releases/?&cursor=100:1:0"
    );
    assert_eq!(b.get("cursor").unwrap(), &"100:1:0");
    assert_eq!(b.get("rel").unwrap(), &"next");
    assert_eq!(b.get("results").unwrap(), &"true");
}
