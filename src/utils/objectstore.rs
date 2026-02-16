//! Utilities to work with the Objectstore service.

use anyhow::Result;

use crate::api::AuthenticatedApi;

pub fn get_objectstore_url(api: &AuthenticatedApi<'_>, org: &str) -> Result<String> {
    let base = api.fetch_organization_details(org)?.links.region_url;
    Ok(format!("{base}/api/0/organizations/{org}/objectstore"))
}
