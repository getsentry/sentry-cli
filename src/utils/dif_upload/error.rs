//! Error types for the dif_upload module.

use indicatif::HumanBytes;
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
    #[error(
        "Debug file's size ({}) exceeds the maximum allowed size ({})",
        HumanBytes(*size as u64),
        HumanBytes(*max_size)
    )]
    TooLarge { size: usize, max_size: u64 },
}

/// Handles a DIF validation error by logging it to console
/// at the appropriate log level.
pub fn handle(dif_name: &str, error: &ValidationError) {
    let message = format!("Skipping {}: {}", dif_name, error);
    match error {
        ValidationError::InvalidFormat
        | ValidationError::InvalidFeatures
        | ValidationError::InvalidDebugId => log::debug!("{message}"),
        ValidationError::TooLarge { .. } => log::warn!("{message}"),
    }
}
