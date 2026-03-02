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
    pub snapshot_url: Option<String>,
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

const RESERVED_KEYS: &[&str] = &["image_file_name", "width", "height"];

impl Serialize for ImageMetadata {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let extra_count = self
            .extra
            .keys()
            .filter(|k| !RESERVED_KEYS.contains(&k.as_str()))
            .count();
        let mut map = serializer.serialize_map(Some(extra_count + 3))?;

        // CLI-managed fields first
        map.serialize_entry("image_file_name", &self.image_file_name)?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;

        // User-provided sidecar fields, skipping any that conflict with CLI fields
        for (key, value) in &self.extra {
            if !RESERVED_KEYS.contains(&key.as_str()) {
                map.serialize_entry(key, value)?;
            }
        }

        map.end()
    }
}
