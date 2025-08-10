use log::debug;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Adaptive sampling strategy for high-volume log scenarios
#[derive(Debug, Clone)]
pub struct AdaptiveSampler {
    /// Current sampling rate (0.0 to 1.0)
    current_rate: f64,
    /// Base sampling rate when load is normal
    base_rate: f64,
    /// Minimum sampling rate (never go below this)
    min_rate: f64,
    /// Maximum sampling rate (never exceed this)
    max_rate: f64,
    /// Target events per second
    target_eps: f64,
    /// Recent event count tracking
    recent_events: Vec<(Instant, usize)>,
    /// Window for tracking recent events
    tracking_window: Duration,
    /// Counter for decisions since last rate adjustment
    decisions_since_adjustment: usize,
    /// How often to recalculate the sampling rate
    adjustment_interval: usize,
}

impl AdaptiveSampler {
    /// Create a new adaptive sampler
    pub fn new(base_rate: f64, target_eps: f64) -> Self {
        AdaptiveSampler {
            current_rate: base_rate,
            base_rate,
            min_rate: 0.01, // Always sample at least 1%
            max_rate: 1.0,  // Never exceed 100%
            target_eps,
            recent_events: Vec::new(),
            tracking_window: Duration::from_secs(60),
            decisions_since_adjustment: 0,
            adjustment_interval: 100,
        }
    }

    /// Decide whether to sample this event
    pub fn should_sample(&mut self) -> bool {
        self.decisions_since_adjustment += 1;

        // Periodically adjust the sampling rate
        if self.decisions_since_adjustment >= self.adjustment_interval {
            self.adjust_sampling_rate();
            self.decisions_since_adjustment = 0;
        }

        // Make sampling decision
        let random_value: f64 = rand::random();
        let should_sample = random_value < self.current_rate;

        if should_sample {
            self.record_sampled_event();
        }

        should_sample
    }

    /// Record that an event was sampled
    fn record_sampled_event(&mut self) {
        let now = Instant::now();
        self.recent_events.push((now, 1));

        // Clean old events outside the tracking window
        let cutoff = now - self.tracking_window;
        self.recent_events
            .retain(|(timestamp, _)| *timestamp > cutoff);
    }

    /// Adjust sampling rate based on recent activity
    fn adjust_sampling_rate(&mut self) {
        let current_eps = self.calculate_current_eps();
        let previous_rate = self.current_rate;

        if current_eps > self.target_eps * 1.2 {
            // Too many events, decrease sampling rate
            self.current_rate = (self.current_rate * 0.8).max(self.min_rate);
            debug!(
                "High load detected ({:.1} eps), reducing sampling rate: {:.3} -> {:.3}",
                current_eps, previous_rate, self.current_rate
            );
        } else if current_eps < self.target_eps * 0.5 && self.current_rate < self.base_rate {
            // Low load, can increase sampling rate back towards base
            self.current_rate = (self.current_rate * 1.2)
                .min(self.base_rate)
                .min(self.max_rate);
            debug!(
                "Low load detected ({:.1} eps), increasing sampling rate: {:.3} -> {:.3}",
                current_eps, previous_rate, self.current_rate
            );
        }
    }

    /// Calculate current events per second
    fn calculate_current_eps(&self) -> f64 {
        if self.recent_events.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let events_in_window: usize = self
            .recent_events
            .iter()
            .filter(|(timestamp, _)| now.duration_since(*timestamp) <= self.tracking_window)
            .map(|(_, count)| count)
            .sum();

        events_in_window as f64 / self.tracking_window.as_secs_f64()
    }

    /// Get current sampling rate
    pub fn current_rate(&self) -> f64 {
        self.current_rate
    }

    /// Get sampling statistics
    #[expect(dead_code)]
    pub fn get_stats(&self) -> SamplingStats {
        SamplingStats {
            current_rate: self.current_rate,
        }
    }
}

/// Statistics about sampling behavior
#[derive(Debug, Clone)]
#[expect(dead_code)]
pub struct SamplingStats {
    pub current_rate: f64,
}

/// Priority-based sampling for different log levels
#[derive(Debug)]
pub struct PrioritySampler {
    /// Sampling rates by log level priority
    level_rates: HashMap<String, f64>,
    /// Default sampling rate for unknown levels
    default_rate: f64,
    /// Adaptive sampler for overall rate control
    adaptive_sampler: AdaptiveSampler,
}

impl PrioritySampler {
    /// Create a new priority sampler
    pub fn new(target_eps: f64) -> Self {
        let mut level_rates = HashMap::new();
        level_rates.insert("fatal".to_owned(), 1.0); // Always sample fatal
        level_rates.insert("error".to_owned(), 0.8); // Sample most errors
        level_rates.insert("warning".to_owned(), 0.4); // Sample some warnings
        level_rates.insert("info".to_owned(), 0.1); // Sample few info
        level_rates.insert("debug".to_owned(), 0.01); // Sample very few debug

        PrioritySampler {
            level_rates,
            default_rate: 0.1,
            adaptive_sampler: AdaptiveSampler::new(0.2, target_eps),
        }
    }

    /// Decide whether to sample based on log level and adaptive sampling
    pub fn should_sample(&mut self, log_level: Option<&str>) -> bool {
        // Get base rate for this log level
        let level_rate = log_level
            .and_then(|level| self.level_rates.get(level))
            .copied()
            .unwrap_or(self.default_rate);

        // Apply adaptive sampling on top of level-based sampling
        let adaptive_factor = self.adaptive_sampler.current_rate();
        let final_rate = level_rate * adaptive_factor;

        // Make sampling decision
        let should_sample = rand::random::<f64>() < final_rate;

        if should_sample {
            // Update adaptive sampler
            self.adaptive_sampler.should_sample();
        }

        should_sample
    }
}

/// Simple deterministic sampling based on hash
#[expect(dead_code)]
pub fn hash_sample(content: &str, rate: f64) -> bool {
    if rate >= 1.0 {
        return true;
    }
    if rate <= 0.0 {
        return false;
    }

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let hash = hasher.finish();

    let threshold = (rate * (u64::MAX as f64)) as u64;
    hash < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_sampler() {
        let mut sampler = AdaptiveSampler::new(0.5, 10.0);

        // Initial rate should be the base rate
        assert!((sampler.current_rate() - 0.5).abs() < 0.01);

        // Should make sampling decisions
        let _ = sampler.should_sample();
        let stats = sampler.get_stats();
        assert!(stats.current_rate > 0.0);
    }

    #[test]
    fn test_priority_sampler() {
        let mut sampler = PrioritySampler::new(10.0);

        // Fatal errors should have higher sampling rate
        let fatal_decisions: Vec<_> = (0..100)
            .map(|_| sampler.should_sample(Some("fatal")))
            .collect();
        let fatal_rate = fatal_decisions.iter().filter(|&&x| x).count() as f64 / 100.0;

        let debug_decisions: Vec<_> = (0..100)
            .map(|_| sampler.should_sample(Some("debug")))
            .collect();
        let debug_rate = debug_decisions.iter().filter(|&&x| x).count() as f64 / 100.0;

        // Fatal should be sampled more often than debug
        assert!(fatal_rate > debug_rate);
    }

    #[test]
    fn test_hash_sample() {
        // Test with 50% rate
        let samples: Vec<_> = (0..1000)
            .map(|i| hash_sample(&format!("entry_{i}"), 0.5))
            .collect();

        let sample_rate = samples.iter().filter(|&&x| x).count() as f64 / 1000.0;

        // Should be roughly 50% (within 10% tolerance)
        assert!((sample_rate - 0.5).abs() < 0.1);

        // Same content should always give same result
        assert_eq!(hash_sample("test", 0.5), hash_sample("test", 0.5));
    }
}
