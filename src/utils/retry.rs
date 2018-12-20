use std::time::{Duration, Instant};

use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;

use crate::constants::{
    DEFAULT_INITIAL_INTERVAL, DEFAULT_MAX_INTERVAL, DEFAULT_MULTIPLIER, DEFAULT_RANDOMIZATION,
};

/// Returns an ExponentialBackoff object instantianted with default values
pub fn get_default_backoff() -> ExponentialBackoff {
    let mut eb = ExponentialBackoff {
        current_interval: Duration::from_millis(DEFAULT_INITIAL_INTERVAL),
        initial_interval: Duration::from_millis(DEFAULT_INITIAL_INTERVAL),
        randomization_factor: DEFAULT_RANDOMIZATION,
        multiplier: DEFAULT_MULTIPLIER,
        max_interval: Duration::from_millis(DEFAULT_MAX_INTERVAL),
        max_elapsed_time: None,
        clock: Default::default(),
        start_time: Instant::now(),
    };
    eb.reset();
    eb
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
