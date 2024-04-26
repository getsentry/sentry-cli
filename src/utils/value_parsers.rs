use std::error::Error;

/// Parse key:value pair from string, used as a value_parser for Clap arguments
pub fn kv_parser(s: &str) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    let pos = s
        .find(':')
        .ok_or_else(|| format!("`{s}` is missing a `:`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
