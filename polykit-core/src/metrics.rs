//! Metrics and observability for task execution.

use std::time::Duration;

/// Metrics collected during task execution.
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    /// Total number of packages processed.
    pub packages_total: usize,
    /// Number of packages that succeeded.
    pub packages_succeeded: usize,
    /// Number of packages that failed.
    pub packages_failed: usize,
    /// Total execution time.
    pub total_duration: Duration,
    /// Duration per package (package name -> duration).
    pub package_durations: std::collections::HashMap<String, Duration>,
    /// Cache hit rate (0.0 to 1.0).
    pub cache_hit_rate: f64,
}

impl ExecutionMetrics {
    /// Creates a new metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a package execution result.
    pub fn record_package(&mut self, package_name: String, duration: Duration, success: bool) {
        self.packages_total += 1;
        if success {
            self.packages_succeeded += 1;
        } else {
            self.packages_failed += 1;
        }
        self.package_durations.insert(package_name, duration);
    }

    /// Sets the total execution duration.
    pub fn set_total_duration(&mut self, duration: Duration) {
        self.total_duration = duration;
    }

    /// Sets the cache hit rate.
    pub fn set_cache_hit_rate(&mut self, rate: f64) {
        self.cache_hit_rate = rate;
    }

    /// Returns the average duration per package.
    pub fn average_package_duration(&self) -> Duration {
        if self.package_durations.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.package_durations.values().sum();
        total / self.package_durations.len() as u32
    }

    /// Returns the success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        if self.packages_total == 0 {
            return 0.0;
        }
        self.packages_succeeded as f64 / self.packages_total as f64
    }
}
