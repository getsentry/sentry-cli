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
    /// When true, this upload contains only a subset of images.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub selective: bool,
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

    fn empty_vcs_info() -> VcsInfo<'static> {
        VcsInfo {
            head_sha: None,
            base_sha: None,
            vcs_provider: "".into(),
            head_repo_name: "".into(),
            base_repo_name: "".into(),
            head_ref: "".into(),
            base_ref: "".into(),
            pr_number: None,
        }
    }

    #[test]
    fn manifest_omits_selective_when_false() {
        let manifest = SnapshotsManifest {
            app_id: "app".into(),
            images: HashMap::new(),
            diff_threshold: None,
            selective: false,
            vcs_info: empty_vcs_info(),
        };
        let json = serde_json::to_value(&manifest).unwrap();
        assert!(!json.as_object().unwrap().contains_key("selective"));
    }

    #[test]
    fn manifest_includes_selective_when_true() {
        let manifest = SnapshotsManifest {
            app_id: "app".into(),
            images: HashMap::new(),
            diff_threshold: None,
            selective: true,
            vcs_info: empty_vcs_info(),
        };
        let json = serde_json::to_value(&manifest).unwrap();
        assert_eq!(json["selective"], json!(true));
    }

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
