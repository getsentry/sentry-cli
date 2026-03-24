//! Data types for the bulk code mappings API.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCodeMappingsRequest<'a> {
    pub project: &'a str,
    pub repository: &'a str,
    pub default_branch: &'a str,
    pub mappings: &'a [BulkCodeMapping],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCodeMapping {
    pub stack_root: String,
    pub source_root: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkCodeMappingsResponse {
    pub created: u64,
    pub updated: u64,
    pub errors: u64,
    pub mappings: Vec<BulkCodeMappingResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCodeMappingResult {
    pub stack_root: String,
    pub source_root: String,
    pub status: String,
    #[serde(default)]
    pub detail: Option<String>,
}
