use log::{debug, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Memory usage monitor for tracking and limiting memory consumption
#[derive(Debug, Clone)]
pub struct MemoryMonitor {
    /// Current estimated memory usage in bytes
    current_usage: Arc<AtomicUsize>,
    /// Maximum allowed memory usage in bytes
    max_usage: usize,
    /// Last time we logged memory stats
    last_log_time: Arc<AtomicUsize>,
    /// Log interval in seconds
    log_interval: Duration,
}

impl MemoryMonitor {
    /// Create a new memory monitor with the specified maximum usage
    pub fn new(max_usage_mb: usize) -> Self {
        MemoryMonitor {
            current_usage: Arc::new(AtomicUsize::new(0)),
            max_usage: max_usage_mb * 1024 * 1024, // Convert MB to bytes
            last_log_time: Arc::new(AtomicUsize::new(0)),
            log_interval: Duration::from_secs(30),
        }
    }

    /// Record memory usage for a log entry
    pub fn record_log_entry(&self, entry_size: usize) -> bool {
        let new_usage = self.current_usage.fetch_add(entry_size, Ordering::Relaxed) + entry_size;
        
        // Check if we're approaching the memory limit
        if new_usage > self.max_usage {
            warn!("Memory usage exceeded limit: {} MB / {} MB", 
                  new_usage / (1024 * 1024), 
                  self.max_usage / (1024 * 1024));
            return false;
        }

        // Log memory stats periodically
        self.maybe_log_stats(new_usage);
        
        true
    }

    /// Record memory release when entries are flushed
    pub fn record_flush(&self, released_bytes: usize) {
        let previous = self.current_usage.fetch_sub(released_bytes, Ordering::Relaxed);
        debug!("Released {} bytes, current usage: {} MB", 
               released_bytes, 
               (previous - released_bytes) / (1024 * 1024));
    }

    /// Get current memory usage in bytes
    pub fn current_usage_bytes(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get current memory usage in MB
    pub fn current_usage_mb(&self) -> usize {
        self.current_usage_bytes() / (1024 * 1024)
    }

    /// Get maximum allowed memory usage in MB
    pub fn max_usage_mb(&self) -> usize {
        self.max_usage / (1024 * 1024)
    }

    /// Check if we're near the memory limit (>80%)
    pub fn is_near_limit(&self) -> bool {
        let current = self.current_usage.load(Ordering::Relaxed);
        current > (self.max_usage * 8) / 10
    }

    /// Get memory usage percentage (0-100)
    pub fn usage_percentage(&self) -> f64 {
        let current = self.current_usage.load(Ordering::Relaxed) as f64;
        let max = self.max_usage as f64;
        (current / max) * 100.0
    }

    /// Maybe log memory statistics if enough time has passed
    fn maybe_log_stats(&self, current_usage: usize) {
        let now = Instant::now().elapsed().as_secs() as usize;
        let last_log = self.last_log_time.load(Ordering::Relaxed);
        
        if now - last_log >= self.log_interval.as_secs() as usize {
            if self.last_log_time.compare_exchange(last_log, now, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                debug!("Memory usage: {} MB / {} MB ({:.1}%)", 
                       current_usage / (1024 * 1024),
                       self.max_usage / (1024 * 1024),
                       self.usage_percentage());
            }
        }
    }
}

/// Estimate the memory footprint of a string entry
pub fn estimate_entry_size(entry: &str) -> usize {
    // Base string size + some overhead for Vec storage and metadata
    entry.len() + std::mem::size_of::<String>() + 32
}

/// Memory-bounded queue for log entries with automatic cleanup
#[derive(Debug)]
pub struct BoundedLogQueue {
    entries: Vec<String>,
    memory_monitor: MemoryMonitor,
    max_entries: usize,
}

impl BoundedLogQueue {
    /// Create a new bounded log queue
    pub fn new(max_memory_mb: usize, max_entries: usize) -> Self {
        BoundedLogQueue {
            entries: Vec::with_capacity(max_entries.min(1000)),
            memory_monitor: MemoryMonitor::new(max_memory_mb),
            max_entries,
        }
    }

    /// Add an entry to the queue, potentially dropping old entries
    pub fn push(&mut self, entry: String) -> bool {
        let entry_size = estimate_entry_size(&entry);
        
        // Check memory limit
        if !self.memory_monitor.record_log_entry(entry_size) {
            // Memory limit exceeded, drop oldest entries
            self.make_room(entry_size);
        }

        // Check entry count limit
        if self.entries.len() >= self.max_entries {
            let removed = self.entries.remove(0);
            let removed_size = estimate_entry_size(&removed);
            self.memory_monitor.record_flush(removed_size);
        }

        self.entries.push(entry);
        true
    }

    /// Make room by removing old entries
    fn make_room(&mut self, needed_bytes: usize) {
        let mut freed_bytes = 0;
        let target_bytes = needed_bytes + (self.memory_monitor.max_usage_mb() * 1024 * 1024) / 10; // Free 10% extra
        
        while freed_bytes < target_bytes && !self.entries.is_empty() {
            let removed = self.entries.remove(0);
            let removed_size = estimate_entry_size(&removed);
            freed_bytes += removed_size;
            self.memory_monitor.record_flush(removed_size);
        }

        warn!("Memory pressure: dropped {} entries to free {} bytes", 
              freed_bytes / estimate_entry_size("average_entry"), freed_bytes);
    }

    /// Drain all entries and update memory tracking
    pub fn drain(&mut self) -> Vec<String> {
        let entries = std::mem::take(&mut self.entries);
        let total_size: usize = entries.iter().map(|e| estimate_entry_size(e)).sum();
        self.memory_monitor.record_flush(total_size);
        entries
    }

    /// Get current queue length
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get memory monitor reference
    pub fn memory_monitor(&self) -> &MemoryMonitor {
        &self.memory_monitor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_monitor() {
        let monitor = MemoryMonitor::new(1); // 1 MB limit
        
        // Should accept small entries
        assert!(monitor.record_log_entry(1000));
        assert_eq!(monitor.current_usage_bytes(), 1000);
        
        // Should reject when limit exceeded
        assert!(!monitor.record_log_entry(2 * 1024 * 1024)); // 2 MB
        
        // Should handle flush correctly
        monitor.record_flush(500);
        assert_eq!(monitor.current_usage_bytes(), 500);
    }

    #[test]
    fn test_bounded_queue() {
        let mut queue = BoundedLogQueue::new(1, 3); // 1 MB, 3 entries max
        
        queue.push("entry1".to_string());
        queue.push("entry2".to_string());
        queue.push("entry3".to_string());
        assert_eq!(queue.len(), 3);
        
        // Should drop oldest when adding 4th entry
        queue.push("entry4".to_string());
        assert_eq!(queue.len(), 3);
        
        let entries = queue.drain();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], "entry2"); // entry1 was dropped
    }
}
