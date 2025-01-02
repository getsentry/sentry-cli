//! Error types for the dif_upload module.

use thiserror::Error;

/// Represents an error that makes a DIF invalid.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Invalid features")]
    InvalidFeatures,
    #[error("Invalid debug ID")]
    InvalidDebugId,
    #[error("Debug file is too large")]
    TooLarge,
}

/// Handles a DIF validation error by logging it to console
/// at the appropriate log level.
pub fn handle(dif_name: &str, error: &ValidationError) {
    log::debug!("Skipping {}: {}", dif_name, error);
}
