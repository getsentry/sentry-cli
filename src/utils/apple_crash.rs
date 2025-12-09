//! Parser for Apple crash reports in .ips format.
//!
//! This module provides functionality to parse Apple crash reports (`.ips` files)
//! in JSON format and convert them to Sentry Event structures for symbolication.
//!
//! Reference: https://developer.apple.com/documentation/xcode/interpreting-the-json-format-of-a-crash-report

use anyhow::Result;
use sentry::protocol::{
    AppleDebugImage, DebugImage, DebugMeta, DeviceContext, Exception, Frame, Level, OsContext,
    Stacktrace, Thread, Values,
};
use sentry::types::Uuid;
use sentry::{protocol::Event, types::protocol::v7::Context as ContextValue};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::utils::event::get_sdk_info;

/// Root structure of an .ips crash report
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpsCrashReport {
    pub incident: Option<String>,
    pub crash_reporter_key: Option<String>,
    pub os_version: Option<String>,
    pub bundle_id: Option<String>,
    pub app_version: Option<String>,
    pub exception: Option<IpsException>,
    pub threads: Option<Vec<IpsThread>>,
    pub used_images: Option<Vec<IpsImage>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpsException {
    #[serde(rename = "type")]
    pub exception_type: Option<String>,
    pub signal: Option<String>,
    pub codes: Option<String>,
    pub subtype: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IpsThread {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub crashed: Option<bool>,
    pub frames: Option<Vec<IpsFrame>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpsFrame {
    pub image_offset: Option<u64>,
    pub image_index: Option<usize>,
    pub symbol: Option<String>,
    pub symbol_location: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct IpsImage {
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub arch: Option<String>,
    pub base: Option<u64>,
}

/// Parse an .ips crash report and convert it to a Sentry Event
pub fn parse_ips_crash_report(content: &str) -> Result<Event<'static>> {
    // Deserialize JSON using serde
    let ips: IpsCrashReport = serde_json::from_str(content)?;

    // Convert to Sentry Event
    let mut event = Event {
        platform: Cow::Borrowed("cocoa"),
        level: Level::Fatal,
        sdk: Some(get_sdk_info()),
        ..Default::default()
    };

    // Extract exception information
    if let Some(exception) = &ips.exception {
        event.exception = convert_exception(exception);
    }

    // Extract thread information
    if let Some(threads) = &ips.threads {
        event.threads = convert_threads(threads, ips.used_images.as_deref());
    }

    // Extract debug images for symbolication
    if let Some(images) = &ips.used_images {
        event.debug_meta = Cow::Owned(convert_debug_images(images));
    }

    // Extract contexts (device, OS, app info)
    event.contexts = convert_contexts(&ips);

    // Set release from app version if available
    if let Some(app_version) = &ips.app_version {
        event.release = Some(Cow::Owned(app_version.clone()));
    }

    Ok(event)
}

/// Convert IPS exception to Sentry Exception
fn convert_exception(ips_exception: &IpsException) -> Values<Exception> {
    let exception_type = ips_exception.exception_type.as_deref().unwrap_or("Unknown");

    // Build exception value from available fields
    let mut value_parts = Vec::new();
    if let Some(signal) = &ips_exception.signal {
        value_parts.push(signal.clone());
    }
    if let Some(codes) = &ips_exception.codes {
        value_parts.push(codes.clone());
    }
    if let Some(subtype) = &ips_exception.subtype {
        value_parts.push(subtype.clone());
    }

    let value = if value_parts.is_empty() {
        exception_type.to_owned()
    } else {
        value_parts.join(" - ")
    };

    let exception = Exception {
        ty: exception_type.into(),
        value: Some(value),
        ..Default::default()
    };

    Values {
        values: vec![exception],
    }
}

/// Convert IPS threads to Sentry Thread objects
fn convert_threads(ips_threads: &[IpsThread], used_images: Option<&[IpsImage]>) -> Values<Thread> {
    let threads = ips_threads
        .iter()
        .filter_map(|thread| {
            let frames = thread.frames.as_ref()?;
            let stacktrace = convert_stacktrace(frames, used_images);

            Some(Thread {
                id: thread.id.map(|id| id.to_string().into()),
                name: thread.name.clone(),
                crashed: thread.crashed.unwrap_or(false),
                stacktrace: Some(stacktrace),
                ..Default::default()
            })
        })
        .collect();

    Values { values: threads }
}

/// Convert IPS frames to Sentry Stacktrace
fn convert_stacktrace(ips_frames: &[IpsFrame], used_images: Option<&[IpsImage]>) -> Stacktrace {
    let frames = ips_frames
        .iter()
        .filter_map(|ips_frame| convert_frame(ips_frame, used_images))
        .rev() // Sentry expects frames in reverse order (innermost first)
        .collect();

    Stacktrace {
        frames,
        ..Default::default()
    }
}

/// Convert IPS frame to Sentry Frame
fn convert_frame(ips_frame: &IpsFrame, used_images: Option<&[IpsImage]>) -> Option<Frame> {
    let image_offset = ips_frame.image_offset?;
    let image_index = ips_frame.image_index?;

    // Get the corresponding binary image
    let image = used_images?.get(image_index)?;
    let base_addr = image.base?;

    // Calculate instruction address
    let instruction_addr = base_addr + image_offset;

    Some(Frame {
        instruction_addr: Some(instruction_addr.into()),
        package: image.name.clone(),
        symbol: ips_frame.symbol.clone(),
        function: ips_frame.symbol.clone(),
        image_addr: Some(base_addr.into()),
        symbol_addr: ips_frame.symbol_location.map(|loc| {
            let symbol_addr = base_addr + image_offset - loc;
            symbol_addr.into()
        }),
        ..Default::default()
    })
}

/// Convert IPS debug images to Sentry DebugMeta
fn convert_debug_images(ips_images: &[IpsImage]) -> DebugMeta {
    let images = ips_images
        .iter()
        .filter_map(|ips_image| {
            let uuid_str = ips_image.uuid.as_ref()?.replace("-", "");
            let uuid = Uuid::parse_str(&uuid_str).ok()?;
            let debug_id = uuid;

            Some(DebugImage::Apple(AppleDebugImage {
                uuid: debug_id,
                image_addr: ips_image.base.map(Into::into)?,
                name: ips_image.name.clone().unwrap_or_default(),
                arch: ips_image.arch.clone(),
                cpu_type: None,
                cpu_subtype: None,
                image_size: 0,
                image_vmaddr: 0u64.into(),
            }))
        })
        .collect();

    DebugMeta {
        images,
        ..Default::default()
    }
}

/// Convert IPS metadata to Sentry Contexts
fn convert_contexts(ips: &IpsCrashReport) -> BTreeMap<String, ContextValue> {
    let mut contexts = BTreeMap::new();

    // Add OS context if available
    if let Some(os_version) = &ips.os_version {
        // Parse OS version string (e.g., "iOS 17.0 (21A329)")
        let parts: Vec<&str> = os_version.split_whitespace().collect();
        let os_name = parts.first().map(|s| s.to_string());
        let os_version_num = parts.get(1).map(|s| s.to_string());

        let mut os_context = OsContext {
            name: os_name,
            version: os_version_num,
            ..Default::default()
        };

        // Add raw description to other field
        os_context.other.insert(
            "raw_description".to_owned(),
            serde_json::Value::String(os_version.clone()),
        );

        contexts.insert("os".into(), ContextValue::Os(Box::new(os_context)));
    }

    // Add device context if available
    if let Some(crash_reporter_key) = &ips.crash_reporter_key {
        let device_context = DeviceContext {
            model_id: Some(crash_reporter_key.clone()),
            ..Default::default()
        };

        contexts.insert(
            "device".into(),
            ContextValue::Device(Box::new(device_context)),
        );
    }

    // Add app context if available
    if ips.bundle_id.is_some() || ips.app_version.is_some() {
        let mut app_data = BTreeMap::new();
        if let Some(bundle_id) = &ips.bundle_id {
            app_data.insert(
                "app_identifier".to_owned(),
                serde_json::Value::String(bundle_id.clone()),
            );
        }
        if let Some(app_version) = &ips.app_version {
            app_data.insert(
                "app_version".to_owned(),
                serde_json::Value::String(app_version.clone()),
            );
        }

        contexts.insert("app".into(), ContextValue::Other(app_data));
    }

    contexts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_crash() {
        let json = r#"{"exception": {"type": "EXC_BAD_ACCESS"}}"#;
        let event = parse_ips_crash_report(json).unwrap();
        assert_eq!(event.platform, Cow::Borrowed("cocoa"));
        assert_eq!(event.level, Level::Fatal);
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        assert!(parse_ips_crash_report(json).is_err());
    }

    #[test]
    fn test_deserialize_ips_report() {
        let json = r#"{"incident": "test", "bundleId": "com.example.app"}"#;
        let ips: IpsCrashReport = serde_json::from_str(json).unwrap();
        assert_eq!(ips.incident.as_deref(), Some("test"));
        assert_eq!(ips.bundle_id.as_deref(), Some("com.example.app"));
    }

    #[test]
    fn test_parse_with_missing_fields() {
        // serde handles missing optional fields
        let json = r#"{}"#;
        let event = parse_ips_crash_report(json).unwrap();
        assert_eq!(event.platform, Cow::Borrowed("cocoa"));
    }

    #[test]
    fn test_parse_complete_crash() {
        let json = r#"{
            "incident": "A1B2C3D4-1234-5678-9ABC-DEF012345678",
            "crashReporterKey": "test-device-key",
            "osVersion": "iOS 17.0 (21A329)",
            "bundleID": "io.sentry.test",
            "appVersion": "1.0.0",
            "exception": {
                "type": "EXC_BAD_ACCESS",
                "signal": "SIGSEGV",
                "codes": "0x0000000000000001",
                "subtype": "KERN_INVALID_ADDRESS at 0x0000000000000000"
            },
            "threads": [{
                "id": 0,
                "crashed": true,
                "frames": [{
                    "imageOffset": 4096,
                    "imageIndex": 0,
                    "symbol": "main",
                    "symbolLocation": 0
                }]
            }],
            "usedImages": [{
                "uuid": "12345678-1234-1234-1234-123456789abc",
                "name": "TestApp",
                "arch": "arm64",
                "base": 4294967296
            }]
        }"#;

        let event = parse_ips_crash_report(json).unwrap();
        assert_eq!(event.platform, Cow::Borrowed("cocoa"));
        assert_eq!(event.level, Level::Fatal);
        assert!(!event.exception.values.is_empty());
        assert!(!event.threads.values.is_empty());
        assert!(!event.debug_meta.images.is_empty());
        assert_eq!(event.release, Some(Cow::Borrowed("1.0.0")));
    }
}
