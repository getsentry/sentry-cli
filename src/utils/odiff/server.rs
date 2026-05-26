use std::io::{BufRead as _, BufReader, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OdiffRequest<'a> {
    request_id: u64,
    base: String,
    compare: String,
    output: String,
    options: &'a OdiffOptions,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OdiffOptions {
    pub threshold: f64,
    pub antialiasing: bool,
    pub output_diff_mask: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OdiffResponse {
    pub request_id: u64,
    #[serde(rename = "match")]
    pub is_match: bool,
    pub reason: Option<String>,
    pub diff_count: Option<u64>,
    pub diff_percentage: Option<f64>,
    pub error: Option<String>,
}

const READ_TIMEOUT: Duration = Duration::from_secs(60);

pub struct OdiffServer {
    child: Child,
    line_rx: mpsc::Receiver<std::io::Result<String>>,
    next_id: u64,
}

fn recv_line(rx: &mpsc::Receiver<std::io::Result<String>>, context: &str) -> Result<String> {
    match rx.recv_timeout(READ_TIMEOUT) {
        Ok(line) => line.context(context.to_owned()),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            bail!("Timed out {context} ({}s)", READ_TIMEOUT.as_secs())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            bail!("odiff process exited unexpectedly while {context}")
        }
    }
}

impl OdiffServer {
    pub fn start(binary_path: &Path) -> Result<Self> {
        let mut child = Command::new(binary_path)
            .arg("--server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn odiff process")?;

        let child_stdout = child
            .stdout
            .take()
            .context("Failed to capture odiff stdout")?;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(child_stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });

        let ready_line = recv_line(&rx, "waiting for odiff ready message")?;

        let ready_value: serde_json::Value = serde_json::from_str(ready_line.trim())
            .context("Failed to parse odiff ready message")?;

        if ready_value.get("ready") != Some(&serde_json::Value::Bool(true)) {
            bail!("odiff server did not send ready message, got: {ready_line}");
        }

        Ok(Self {
            child,
            line_rx: rx,
            next_id: 1,
        })
    }

    pub fn compare(
        &mut self,
        base: &Path,
        compare: &Path,
        output: &Path,
        options: &OdiffOptions,
    ) -> Result<OdiffResponse> {
        let request_id = self.next_id;
        self.next_id += 1;

        let request = OdiffRequest {
            request_id,
            base: base.to_string_lossy().into_owned(),
            compare: compare.to_string_lossy().into_owned(),
            output: output.to_string_lossy().into_owned(),
            options,
        };

        let stdin = self
            .child
            .stdin
            .as_mut()
            .context("odiff stdin not available")?;

        let mut json = serde_json::to_string(&request).context("Failed to serialize request")?;
        json.push('\n');
        stdin
            .write_all(json.as_bytes())
            .context("Failed to write to odiff stdin")?;
        stdin.flush().context("Failed to flush odiff stdin")?;

        let response_line = recv_line(&self.line_rx, "reading odiff response")?;

        let response: OdiffResponse =
            serde_json::from_str(response_line.trim()).context("Failed to parse odiff response")?;

        if response.request_id != request_id {
            bail!(
                "odiff response ID mismatch: expected {request_id}, got {}",
                response.request_id
            );
        }

        Ok(response)
    }
}

impl Drop for OdiffServer {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let options = OdiffOptions {
            threshold: 0.1,
            antialiasing: true,
            output_diff_mask: false,
        };
        let request = OdiffRequest {
            request_id: 1,
            base: "/a/base.png".to_owned(),
            compare: "/a/compare.png".to_owned(),
            output: "/a/output.png".to_owned(),
            options: &options,
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(json.contains("\"requestId\""));
        assert!(json.contains("\"outputDiffMask\""));
        assert!(json.contains("\"threshold\""));
    }

    #[test]
    fn test_response_deserialization_match() {
        let json = r#"{"requestId":1,"match":true}"#;
        let response: OdiffResponse =
            serde_json::from_str(json).expect("deserialization should succeed");
        assert_eq!(response.request_id, 1);
        assert!(response.is_match);
        assert!(response.reason.is_none());
        assert!(response.diff_count.is_none());
    }

    #[test]
    fn test_response_deserialization_diff() {
        let json = r#"{"requestId":2,"match":false,"reason":"pixel-diff","diffCount":42,"diffPercentage":1.5}"#;
        let response: OdiffResponse =
            serde_json::from_str(json).expect("deserialization should succeed");
        assert_eq!(response.request_id, 2);
        assert!(!response.is_match);
        assert_eq!(response.reason.as_deref(), Some("pixel-diff"));
        assert_eq!(response.diff_count, Some(42));
        assert!(
            (response.diff_percentage.expect("should have percentage") - 1.5).abs() < f64::EPSILON
        );
    }

    #[test]
    fn test_response_deserialization_layout() {
        let json = r#"{"requestId":3,"match":false,"reason":"layout-diff"}"#;
        let response: OdiffResponse =
            serde_json::from_str(json).expect("deserialization should succeed");
        assert!(!response.is_match);
        assert_eq!(response.reason.as_deref(), Some("layout-diff"));
        assert!(response.diff_count.is_none());
    }

    #[test]
    fn test_response_deserialization_error() {
        let json = r#"{"requestId":4,"match":false,"error":"file not found"}"#;
        let response: OdiffResponse =
            serde_json::from_str(json).expect("deserialization should succeed");
        assert_eq!(response.error.as_deref(), Some("file not found"));
    }
}
