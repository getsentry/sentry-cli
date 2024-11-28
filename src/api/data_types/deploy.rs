//! The `Deploy` data type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Deploy<'d> {
    #[serde(rename = "environment")]
    pub env: Cow<'d, str>,
    pub name: Option<Cow<'d, str>>,
    pub url: Option<Cow<'d, str>>,
    #[serde(rename = "dateStarted")]
    pub started: Option<DateTime<Utc>>,
    #[serde(rename = "dateFinished")]
    pub finished: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<Cow<'d, str>>>,
}

impl Deploy<'_> {
    /// Returns the name of this deploy, defaulting to `"unnamed"`.
    pub fn name(&self) -> &str {
        match self.name.as_deref() {
            Some("") | None => "unnamed",
            Some(name) => name,
        }
    }
}
