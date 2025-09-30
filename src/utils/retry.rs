use std::time::Duration;

use backon::ExponentialBuilder;

use crate::constants::{DEFAULT_INITIAL_INTERVAL, DEFAULT_MAX_INTERVAL, DEFAULT_MULTIPLIER};

/// Returns an exponential backoff builder instantianted with default values
pub fn get_default_backoff() -> ExponentialBuilder {
    ExponentialBuilder::new()
        .with_min_delay(Duration::from_millis(DEFAULT_INITIAL_INTERVAL))
        .with_max_delay(Duration::from_millis(DEFAULT_MAX_INTERVAL))
        .with_jitter()
        .with_factor(DEFAULT_MULTIPLIER)
}

/// Trait for displaying duration-like in milliseconds
pub trait DurationAsMilliseconds {
    fn as_milliseconds(&self) -> u64;
}

impl DurationAsMilliseconds for Duration {
    fn as_milliseconds(&self) -> u64 {
        self.as_secs() * 1000 + u64::from(self.subsec_millis())
    }
}
