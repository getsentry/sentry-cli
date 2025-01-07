//! Error types for the dif_upload module.

use anyhow::Result;
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
/// at the appropriate log level. Or, if the error should stop
/// the upload, it will return an error, that can be propagated
/// to the caller.
pub fn handle(dif_name: &str, error: &ValidationError) -> Result<()> {
    let message = format!("{}: {}", dif_name, error);
    match error {
        ValidationError::InvalidFormat
        | ValidationError::InvalidFeatures
        | ValidationError::InvalidDebugId => log::debug!("Skipping {message}"),
        ValidationError::TooLarge { .. } => {
            anyhow::bail!("Upload failed due to error in debug file {message}")
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(ValidationError::InvalidFormat)]
    #[case(ValidationError::InvalidFeatures)]
    #[case(ValidationError::InvalidDebugId)]
    fn test_handle_should_not_error(#[case] error: ValidationError) {
        let result = handle("test", &error);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_should_error() {
        let error = ValidationError::TooLarge {
            size: 1000,
            max_size: 100,
        };
        let result = handle("test", &error);
        assert!(result.is_err());
    }
}
