//! Utilities to work with the Objectstore service.

use anyhow::Result;

use crate::api::Api;

pub fn get_objectstore_url(api: impl AsRef<Api>, org: &str) -> Result<String> {
    let api = api.as_ref().authenticated()?;
    let base = api.fetch_organization_details(org)?.links.region_url;
    Ok(format!("{base}/api/0/organizations/{org}/objectstore"))
}
