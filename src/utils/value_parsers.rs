use anyhow::{anyhow, Result};

/// Parse key:value pair from string, used as a value_parser for Clap arguments
pub fn kv_parser(s: &str) -> Result<(String, String)> {
    let pos = s
        .find(':')
        .ok_or_else(|| anyhow!(format!("`{s}` is missing a `:`")))?;
    Ok((
        s[..pos].parse().expect("infallible"),
        s[pos + 1..].parse().expect("infallible"),
    ))
}
