//! Apple crash report (.ips) parsing utilities.
//!
//! This module provides functionality to parse Apple's JSON-format crash reports
//! (`.ips` files) and convert them into Sentry events for processing and symbolication.

use anyhow::{Context as _, Result};
use sentry::protocol::{
    AppleDebugImage, DebugImage, DebugMeta, Event, Exception, Frame, Level, Mechanism, Stacktrace,
    Thread, Values,
};
use sentry::types::Uuid;
use serde::Deserialize;
use std::borrow::Cow;
use symbolic::common::DebugId;

/// Root structure of an .ips crash report
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpsCrashReport {
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
    let ips: IpsCrashReport =
        serde_json::from_str(content).context("Failed to parse .ips JSON format")?;

    // Start with a basic event
    let mut event = Event {
        platform: Cow::Borrowed("cocoa"),
        level: Level::Fatal,
        event_id: Uuid::new_v4(),
        ..Default::default()
    };

    // Extract exception information
    if let Some(exception) = &ips.exception {
        event.exception = convert_exception(exception);
    }

    // Extract threads and stacktraces
    if let Some(threads) = &ips.threads {
        event.threads = convert_threads(threads, &ips.used_images);
    }

    // Extract debug images for symbolication
    if let Some(images) = &ips.used_images {
        event.debug_meta = Cow::Owned(convert_debug_meta(images));
    }

    // Set release from app version if available
    if let Some(app_version) = &ips.app_version {
        event.release = Some(Cow::Owned(app_version.clone()));
    }

    Ok(event)
}

/// Convert IPS exception to Sentry exception format
fn convert_exception(ips_exception: &IpsException) -> Values<Exception> {
    let exception_type = ips_exception
        .exception_type
        .clone()
        .unwrap_or_else(|| "Unknown".to_owned());

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
        None
    } else {
        Some(value_parts.join(" - "))
    };

    let mechanism = Mechanism::default();

    Values {
        values: vec![Exception {
            ty: exception_type,
            value,
            mechanism: Some(mechanism),
            ..Default::default()
        }],
    }
}

/// Convert IPS threads to Sentry thread format
fn convert_threads(
    ips_threads: &[IpsThread],
    used_images: &Option<Vec<IpsImage>>,
) -> Values<Thread> {
    let threads: Vec<Thread> = ips_threads
        .iter()
        .map(|thread| {
            let stacktrace = thread.frames.as_ref().map(|frames| convert_stacktrace(frames, used_images));

            Thread {
                id: thread.id.map(|id| id.to_string().into()),
                name: thread.name.clone(),
                crashed: thread.crashed.unwrap_or(false),
                stacktrace,
                ..Default::default()
            }
        })
        .collect();

    Values { values: threads }
}

/// Convert IPS frames to Sentry stacktrace
fn convert_stacktrace(ips_frames: &[IpsFrame], used_images: &Option<Vec<IpsImage>>) -> Stacktrace {
    let frames: Vec<Frame> = ips_frames
        .iter()
        .filter_map(|frame| {
            let image_offset = frame.image_offset?;
            let image_index = frame.image_index?;

            // Get the image information
            let (base_addr, image_name, image_addr) = if let Some(images) = used_images {
                if let Some(image) = images.get(image_index) {
                    let base = image.base.unwrap_or(0);
                    let name = image.name.clone().unwrap_or_else(|| "Unknown".to_owned());
                    (base, name, Some(base.into()))
                } else {
                    (0, "Unknown".to_owned(), None)
                }
            } else {
                (0, "Unknown".to_owned(), None)
            };

            // Calculate absolute instruction address
            let instruction_addr = base_addr + image_offset;

            Some(Frame {
                instruction_addr: Some(instruction_addr.into()),
                package: Some(image_name),
                symbol: frame.symbol.clone(),
                function: frame.symbol.clone(),
                image_addr,
                ..Default::default()
            })
        })
        .rev() // Reverse to match Sentry's stack order (innermost first)
        .collect();

    Stacktrace {
        frames,
        ..Default::default()
    }
}

/// Convert IPS debug images to Sentry debug meta
fn convert_debug_meta(ips_images: &[IpsImage]) -> DebugMeta {
    let images: Vec<DebugImage> = ips_images
        .iter()
        .filter_map(|image| {
            let uuid_str = image.uuid.as_ref()?;
            let uuid = Uuid::parse_str(uuid_str).ok()?;
            let debug_id = DebugId::from_uuid(uuid);

            Some(DebugImage::Apple(AppleDebugImage {
                uuid: debug_id.uuid(),
                image_addr: image.base.unwrap_or(0).into(),
                name: image.name.clone().unwrap_or_else(|| "Unknown".to_owned()),
                arch: image.arch.clone(),
                image_size: 0,
                image_vmaddr: 0.into(),
                cpu_type: Some(0),
                cpu_subtype: Some(0),
            }))
        })
        .collect();

    DebugMeta {
        images,
        ..Default::default()
    }
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
        let json = r#"{"appVersion": "1.0.0", "bundleID": "com.example.app"}"#;
        let ips: IpsCrashReport = serde_json::from_str(json).unwrap();
        assert_eq!(ips.app_version.unwrap(), "1.0.0");
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
            "osVersion": "iOS 17.0",
            "bundleID": "io.sentry.test",
            "appVersion": "1.0.0",
            "exception": {
                "type": "EXC_BAD_ACCESS",
                "signal": "SIGSEGV",
                "codes": "0x0000000000000001",
                "subtype": "KERN_INVALID_ADDRESS"
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
    }
}
