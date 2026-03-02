//! Data types for the snapshots API.

use std::collections::HashMap;

use serde::ser::{SerializeMap as _, Serializer};
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
///
/// Serializes as a flat JSON object. User-provided sidecar fields are included
/// first, then CLI-managed fields (`image_file_name`, `width`, `height`) are
/// written last so they always take precedence.
#[derive(Debug)]
pub struct ImageMetadata {
    pub image_file_name: String,
    pub width: u32,
    pub height: u32,
    pub extra: HashMap<String, serde_json::Value>,
}

impl Serialize for ImageMetadata {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.extra.len() + 3))?;

        // Sidecar fields first (user-provided extras)
        for (key, value) in &self.extra {
            map.serialize_entry(key, value)?;
        }

        // CLI-managed fields last — these always win
        map.serialize_entry("image_file_name", &self.image_file_name)?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;

        map.end()
    }
}
