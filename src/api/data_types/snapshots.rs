//! Data types for the snapshots API.

use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1_smol::Digest;

const IMAGE_FILE_NAME_FIELD: &str = "image_file_name";
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
    // VCS info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_sha: Option<Digest>,
    #[serde(skip_serializing_if = "str::is_empty", rename = "provider")]
    pub vcs_provider: Cow<'a, str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub head_repo_name: Cow<'a, str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub base_repo_name: Cow<'a, str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub head_ref: Cow<'a, str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub base_ref: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u32>,
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
    pub fn new(
        image_file_name: String,
        width: u32,
        height: u32,
        mut extra: HashMap<String, Value>,
    ) -> Self {
        extra.insert(
            IMAGE_FILE_NAME_FIELD.to_owned(),
            Value::String(image_file_name),
        );
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
            (IMAGE_FILE_NAME_FIELD): "from-sidecar.png",
            (WIDTH_FIELD): 1,
            (HEIGHT_FIELD): 2,
            "custom": "keep-me"
        }))
        .unwrap();

        let metadata = ImageMetadata::new("from-cli.png".to_owned(), 100, 200, extra);
        let serialized = serde_json::to_value(metadata).unwrap();

        let expected = json!({
            (IMAGE_FILE_NAME_FIELD): "from-cli.png",
            (WIDTH_FIELD): 100,
            (HEIGHT_FIELD): 200,
            "custom": "keep-me"
        });

        assert_eq!(serialized, expected);
    }
}
