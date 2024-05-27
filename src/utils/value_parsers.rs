use anyhow::{anyhow, Result};

/// Parse key:value pair from string, used as a value_parser for Clap arguments
pub fn kv_parser(s: &str) -> Result<(String, String)> {
    s.split_once(':')
        .map(|(k, v)| (k.into(), v.into()))
        .ok_or_else(|| anyhow!("`{s}` is missing a `:`"))
}
