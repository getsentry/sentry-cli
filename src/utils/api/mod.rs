//! Utilities that rely on the Sentry API.

use crate::api::{Api, AuthenticatedApi};
use anyhow::{Context, Result};

/// Given an org and project slugs or IDs, returns the IDs of both.
pub fn get_org_project_id(api: impl AsRef<Api>, org: &str, project: &str) -> Result<(u64, u64)> {
    let authenticated_api = api.as_ref().authenticated()?;
    let org_id = get_org_id(authenticated_api, org)?;
    let authenticated_api = api.as_ref().authenticated()?;
    let project_id = get_project_id(authenticated_api, org, project)?;
    Ok((org_id, project_id))
}

/// Given an org slug or ID, returns its ID as a number.
fn get_org_id(api: AuthenticatedApi<'_>, org: &str) -> Result<u64> {
    if let Ok(id) = org.parse::<u64>() {
        return Ok(id);
    }
    let details = api.fetch_organization_details(org)?;
    Ok(details
        .id
        .parse::<u64>()
        .context("Unable to parse org id")?)
}

/// Given an org and project slugs or IDs, returns the project ID.
fn get_project_id(api: AuthenticatedApi<'_>, org: &str, project: &str) -> Result<u64> {
    if let Ok(id) = project.parse::<u64>() {
        return Ok(id);
    }

    let projects = api.list_organization_projects(org)?;
    for p in projects {
        if p.slug == project {
            return p.id.parse::<u64>().context("Unable to parse project id");
        }
    }

    anyhow::bail!("Project not found")
}
