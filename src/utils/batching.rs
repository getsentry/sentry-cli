use log::debug;
use std::time::{Duration, Instant};

/// A batching utility for collecting log entries before sending to Sentry
#[derive(Debug)]
pub struct LogBatch {
    entries: Vec<String>,
    max_size: usize,
    timeout: Duration,
    last_flush: Instant,
    adaptive_config: AdaptiveBatchingConfig,
}

/// Configuration for adaptive batching behavior
#[derive(Debug, Clone)]

pub struct AdaptiveBatchingConfig {
    /// Minimum batch size (always flush at least this many)
    pub min_batch_size: usize,
    /// Maximum batch size (never exceed this)
    pub max_batch_size: usize,

    /// Minimum timeout (for high-volume periods)
    pub min_timeout: Duration,
    /// Maximum timeout (for low-volume periods)
    pub max_timeout: Duration,
    /// Track recent flush rates for adaptation
    pub recent_flush_times: Vec<Instant>,
    /// Maximum number of recent flush times to track
    pub max_history: usize,
}

impl Default for AdaptiveBatchingConfig {
    fn default() -> Self {
        AdaptiveBatchingConfig {
            min_batch_size: 10,
            max_batch_size: 1000,

            min_timeout: Duration::from_secs(1),
            max_timeout: Duration::from_secs(30),
            recent_flush_times: Vec::new(),
            max_history: 10,
        }
    }
}

impl LogBatch {
    /// Create a new log batch with specified maximum size and timeout
    #[allow(dead_code)]
    pub fn new(max_size: usize, timeout: Duration) -> Self {
        LogBatch {
            entries: Vec::with_capacity(max_size),
            max_size,
            timeout,
            last_flush: Instant::now(),
            adaptive_config: AdaptiveBatchingConfig::default(),
        }
    }

    /// Create a new adaptive log batch with custom configuration
    pub fn new_adaptive(
        max_size: usize,
        timeout: Duration,
        adaptive_config: AdaptiveBatchingConfig,
    ) -> Self {
        LogBatch {
            entries: Vec::with_capacity(max_size.min(adaptive_config.max_batch_size)),
            max_size,
            timeout,
            last_flush: Instant::now(),
            adaptive_config,
        }
    }

    /// Add a log entry to the batch
    /// Returns true if the batch should be flushed (due to size or timeout)
    pub fn add_entry(&mut self, entry: String) -> bool {
        self.entries.push(entry);

        debug!(
            "Added log entry to batch ({}/{})",
            self.entries.len(),
            self.max_size
        );

        self.should_flush()
    }

    /// Check if the batch should be flushed due to size or timeout
    pub fn should_flush(&self) -> bool {
        let current_timeout = self.get_adaptive_timeout();
        let size_threshold = self.get_adaptive_batch_size();

        self.entries.len() >= size_threshold || self.last_flush.elapsed() >= current_timeout
    }

    /// Get all entries and clear the batch
    pub fn flush(&mut self) -> Vec<String> {
        let entries = std::mem::take(&mut self.entries);
        let now = Instant::now();

        // Record flush time for adaptive behavior
        self.adaptive_config.recent_flush_times.push(now);
        if self.adaptive_config.recent_flush_times.len() > self.adaptive_config.max_history {
            self.adaptive_config.recent_flush_times.remove(0);
        }

        self.last_flush = now;

        debug!("Flushed batch with {} entries", entries.len());

        entries
    }

    /// Calculate adaptive timeout based on recent flush patterns
    fn get_adaptive_timeout(&self) -> Duration {
        if self.adaptive_config.recent_flush_times.len() < 2 {
            return self.timeout;
        }

        // Calculate average time between flushes
        let recent_times = &self.adaptive_config.recent_flush_times;
        let mut total_interval = Duration::from_secs(0);
        for i in 1..recent_times.len() {
            total_interval += recent_times[i].duration_since(recent_times[i - 1]);
        }

        let avg_interval = total_interval / (recent_times.len() - 1) as u32;

        // Adapt timeout based on flush frequency
        // If flushing frequently (high volume), use shorter timeout
        // If flushing infrequently (low volume), use longer timeout
        if avg_interval < Duration::from_secs(2) {
            // High volume - decrease timeout
            self.adaptive_config.min_timeout
        } else if avg_interval > Duration::from_secs(10) {
            // Low volume - increase timeout
            self.adaptive_config.max_timeout
        } else {
            // Normal volume - use base timeout
            self.timeout
        }
    }

    /// Calculate adaptive batch size based on recent patterns
    fn get_adaptive_batch_size(&self) -> usize {
        if self.adaptive_config.recent_flush_times.len() < 2 {
            return self.max_size;
        }

        // If we're flushing very frequently, increase batch size to be more efficient
        let recent_times = &self.adaptive_config.recent_flush_times;
        if recent_times.len() >= 3 {
            let last_intervals: Vec<_> = recent_times
                .windows(2)
                .take(3)
                .map(|w| w[1].duration_since(w[0]))
                .collect();

            let avg_recent_interval =
                last_intervals.iter().sum::<Duration>() / last_intervals.len() as u32;

            if avg_recent_interval < Duration::from_secs(1) {
                // Very high volume - increase batch size for efficiency
                (self.max_size * 2).min(self.adaptive_config.max_batch_size)
            } else if avg_recent_interval > Duration::from_secs(20) {
                // Very low volume - decrease batch size for responsiveness
                self.adaptive_config.min_batch_size
            } else {
                self.max_size
            }
        } else {
            self.max_size
        }
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_size_limit() {
        let mut batch = LogBatch::new(2, Duration::from_secs(60));

        assert!(!batch.add_entry("entry 1".into()));
        assert!(batch.add_entry("entry 2".into()));

        let entries = batch.flush();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "entry 1");
        assert_eq!(entries[1], "entry 2");
        assert!(batch.is_empty());
    }

    #[test]
    fn test_batch_timeout() {
        let mut batch = LogBatch::new(10, Duration::from_millis(1));

        batch.add_entry("entry 1".into());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(2));

        assert!(batch.should_flush());
    }
}
