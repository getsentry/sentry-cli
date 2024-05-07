use anyhow::{anyhow, Result};

// Parse the value argument for a set metric.
pub fn set_value_parser(s: &str) -> Result<i64> {
    match s.parse::<f64>() {
        Ok(_) => Err(anyhow!(format!("floats are not supported for set metrics"))),
        Err(_) => match s.parse::<i64>() {
            Ok(res) => Ok(res),
            Err(_) => Ok(crc32fast::hash(s.as_bytes()) as i64),
        },
    }
}

/// Parse the value argument for a set metric.
pub fn key_parser(s: &str) -> Result<String> {
    if s.chars()
        .next()
        .ok_or_else(|| anyhow!(format!("metric name cannot be empty")))?
        .is_ascii_alphabetic()
    {
        Ok(s.to_string())
    } else {
        Err(anyhow!(format!(
            "metric name must start with an alphabetic character"
        )))
    }
}
