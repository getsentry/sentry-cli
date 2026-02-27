//! Data types for the snapshots API.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Response from the create snapshot endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotResponse {
    pub artifact_id: String,
    pub image_count: u64,
}

// Keep in sync with https://github.com/getsentry/sentry/blob/master/src/sentry/preprod/snapshots/manifest.py
/// Manifest describing a set of snapshot images for an app.
#[derive(Debug, Serialize)]
pub struct SnapshotsManifest {
    pub app_id: String,
    pub images: HashMap<String, ImageMetadata>,
}

// Keep in sync with https://github.com/getsentry/sentry/blob/master/src/sentry/preprod/snapshots/manifest.py
/// Metadata for a single image in a snapshot manifest.
#[derive(Debug, Serialize)]
pub struct ImageMetadata {
    pub image_file_name: String,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
