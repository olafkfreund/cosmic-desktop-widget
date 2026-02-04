//! Performance metrics tracking for the desktop widget
//!
//! This module provides structures for tracking render performance,
//! cache efficiency, and memory usage to ensure the widget stays
//! within performance budgets.

use std::time::{Duration, Instant};

/// Performance targets
pub const TARGET_RENDER_TIME_MS: u64 = 16; // 60fps budget
pub const TARGET_IDLE_CPU_PERCENT: f64 = 0.1;
pub const TARGET_ACTIVE_CPU_PERCENT: f64 = 1.0;
pub const TARGET_MEMORY_MB: u64 = 50;

/// Tracks render performance metrics
#[derive(Debug)]
pub struct RenderMetrics {
    last_render_time: Duration,
    avg_render_time: Duration,
    max_render_time: Duration,
    render_count: u64,
    frames_over_budget: u64,
}

impl RenderMetrics {
    pub fn new() -> Self {
        Self {
            last_render_time: Duration::ZERO,
            avg_render_time: Duration::ZERO,
            max_render_time: Duration::ZERO,
            render_count: 0,
            frames_over_budget: 0,
        }
    }

    /// Record a render duration
    pub fn record_render(&mut self, duration: Duration) {
        self.last_render_time = duration;
        self.render_count += 1;

        // Update max
        if duration > self.max_render_time {
            self.max_render_time = duration;
        }

        // Running average
        let total = self.avg_render_time.as_nanos() * (self.render_count - 1) as u128
            + duration.as_nanos();
        self.avg_render_time = Duration::from_nanos((total / self.render_count as u128) as u64);

        // Track frames over budget
        if duration.as_millis() > TARGET_RENDER_TIME_MS as u128 {
            self.frames_over_budget += 1;
        }
    }

    /// Get average render time
    pub fn avg_render_time(&self) -> Duration {
        self.avg_render_time
    }

    /// Get last render time
    pub fn last_render_time(&self) -> Duration {
        self.last_render_time
    }

    /// Get maximum render time observed
    pub fn max_render_time(&self) -> Duration {
        self.max_render_time
    }

    /// Get total render count
    pub fn render_count(&self) -> u64 {
        self.render_count
    }

    /// Get percentage of frames over budget
    pub fn frames_over_budget_percent(&self) -> f64 {
        if self.render_count == 0 {
            0.0
        } else {
            (self.frames_over_budget as f64 / self.render_count as f64) * 100.0
        }
    }

    /// Check if render time exceeds target
    pub fn is_over_budget(&self) -> bool {
        self.last_render_time.as_millis() > TARGET_RENDER_TIME_MS as u128
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for RenderMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks cache performance metrics
#[derive(Debug)]
pub struct CacheMetrics {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Record cache evictions
    pub fn record_eviction(&mut self, count: u64) {
        self.evictions += count;
    }

    /// Get cache hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Get total hits
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get total misses
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Get total evictions
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple timer for measuring operation duration
#[derive(Debug)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time since timer started
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Stop timer and return elapsed duration
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }
}

/// Aggregated performance metrics for the entire widget
#[derive(Debug, Default)]
pub struct WidgetMetrics {
    pub render: RenderMetrics,
    pub glyph_cache: CacheMetrics,
    last_report: Option<Instant>,
}

impl WidgetMetrics {
    pub fn new() -> Self {
        Self {
            render: RenderMetrics::new(),
            glyph_cache: CacheMetrics::new(),
            last_report: None,
        }
    }

    /// Log metrics summary if enough time has passed (every 60 seconds)
    pub fn maybe_log_summary(&mut self) {
        let should_log = match self.last_report {
            None => true,
            Some(last) => last.elapsed() >= Duration::from_secs(60),
        };

        if should_log && self.render.render_count > 0 {
            self.log_summary();
            self.last_report = Some(Instant::now());
        }
    }

    /// Log a summary of all metrics
    pub fn log_summary(&self) {
        tracing::debug!(
            render_count = %self.render.render_count(),
            avg_render_ms = %self.render.avg_render_time().as_secs_f64() * 1000.0,
            max_render_ms = %self.render.max_render_time().as_secs_f64() * 1000.0,
            frames_over_budget_pct = %self.render.frames_over_budget_percent(),
            cache_hit_rate_pct = %self.glyph_cache.hit_rate(),
            cache_hits = %self.glyph_cache.hits(),
            cache_misses = %self.glyph_cache.misses(),
            cache_evictions = %self.glyph_cache.evictions(),
            "Performance metrics summary"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_metrics() {
        let mut metrics = RenderMetrics::new();

        metrics.record_render(Duration::from_millis(10));
        assert_eq!(metrics.avg_render_time(), Duration::from_millis(10));
        assert_eq!(metrics.render_count(), 1);
        assert!(!metrics.is_over_budget());

        metrics.record_render(Duration::from_millis(20));
        assert_eq!(metrics.avg_render_time(), Duration::from_millis(15));
        assert_eq!(metrics.render_count(), 2);
    }

    #[test]
    fn test_render_over_budget() {
        let mut metrics = RenderMetrics::new();

        metrics.record_render(Duration::from_millis(5));
        assert_eq!(metrics.frames_over_budget_percent(), 0.0);

        metrics.record_render(Duration::from_millis(20)); // Over 16ms budget
        assert_eq!(metrics.frames_over_budget_percent(), 50.0);
    }

    #[test]
    fn test_cache_metrics() {
        let mut metrics = CacheMetrics::new();

        metrics.record_hit();
        metrics.record_hit();
        metrics.record_miss();

        assert_eq!(metrics.hits(), 2);
        assert_eq!(metrics.misses(), 1);
        assert!((metrics.hit_rate() - 66.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_hit_rate_empty() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_widget_metrics() {
        let mut metrics = WidgetMetrics::new();

        metrics.render.record_render(Duration::from_millis(10));
        metrics.glyph_cache.record_hit();
        metrics.glyph_cache.record_miss();

        assert_eq!(metrics.render.render_count(), 1);
        assert_eq!(metrics.glyph_cache.hit_rate(), 50.0);
    }
}
