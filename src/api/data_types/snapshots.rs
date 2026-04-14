//! Data types for the snapshots API.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::VcsInfo;

const WIDTH_FIELD: &str = "width";
const HEIGHT_FIELD: &str = "height";

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
pub struct SnapshotsManifest<'a> {
    pub app_id: String,
    pub images: HashMap<String, ImageMetadata>,
    /// If set, Sentry will only report images as changed if their difference %
    /// is greater than this value (e.g. 0.01 = only report changes >= 1%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_threshold: Option<f64>,
    /// Full list of expected preview names. When provided, images in this list
    /// but absent from the upload are reported as "skipped" instead of "removed".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_image_names: Option<Vec<String>>,
    #[serde(flatten)]
    pub vcs_info: VcsInfo<'a>,
}

// Keep in sync with https://github.com/getsentry/sentry/blob/master/src/sentry/preprod/snapshots/manifest.py
/// Metadata for a single image in a snapshot manifest.
///
/// CLI-managed fields (`image_file_name`, `width`, `height`) override any
/// identically named fields provided by user sidecar metadata.
#[derive(Debug, Serialize)]
pub struct ImageMetadata {
    #[serde(flatten)]
    data: HashMap<String, Value>,
}

impl ImageMetadata {
    pub fn new(width: u32, height: u32, mut extra: HashMap<String, Value>) -> Self {
        extra.insert(WIDTH_FIELD.to_owned(), Value::from(width));
        extra.insert(HEIGHT_FIELD.to_owned(), Value::from(height));

        Self { data: extra }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn cli_managed_fields_override_sidecar_fields() {
        let extra = serde_json::from_value(json!({
            (WIDTH_FIELD): 1,
            (HEIGHT_FIELD): 2,
            "custom": "keep-me"
        }))
        .unwrap();

        let metadata = ImageMetadata::new(100, 200, extra);
        let serialized = serde_json::to_value(metadata).unwrap();

        let expected = json!({
            (WIDTH_FIELD): 100,
            (HEIGHT_FIELD): 200,
            "custom": "keep-me"
        });

        assert_eq!(serialized, expected);
    }
}
