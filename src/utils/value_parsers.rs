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

/// Parse the value argument for a set metric. Floats are floored and strings are hashed.
pub fn set_value_parser(s: &str) -> Result<f64> {
    match s.parse::<f64>() {
        Ok(res) => Ok(res.floor()),
        Err(_) => Ok(crc32fast::hash(s.as_bytes()) as f64),
    }
}
