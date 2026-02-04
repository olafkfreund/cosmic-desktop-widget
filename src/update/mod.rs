//! Update coordination system for widgets

use std::time::{Duration, Instant};

/// Tracks what needs to be updated
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateFlags {
    pub clock: bool,
    pub weather: bool,
    pub layout: bool,
    pub theme: bool,
}

impl UpdateFlags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn needs_redraw(&self) -> bool {
        self.clock || self.weather || self.layout || self.theme
    }

    pub fn clear(&mut self) {
        self.clock = false;
        self.weather = false;
        self.layout = false;
        self.theme = false;
    }

    pub fn set_all(&mut self) {
        self.clock = true;
        self.weather = true;
        self.layout = true;
        self.theme = true;
    }
}

/// Manages update timing for different components
pub struct UpdateScheduler {
    /// Last time clock was updated
    last_clock_update: Instant,

    /// Last time weather was updated
    last_weather_update: Instant,

    /// Clock update interval
    clock_interval: Duration,

    /// Weather update interval
    weather_interval: Duration,

    /// Pending updates
    pending: UpdateFlags,
}

impl UpdateScheduler {
    pub fn new(clock_interval: Duration, weather_interval: Duration) -> Self {
        let now = Instant::now();
        Self {
            last_clock_update: now,
            last_weather_update: now,
            clock_interval,
            weather_interval,
            pending: UpdateFlags::default(),
        }
    }

    /// Check what needs to be updated and return flags
    pub fn check_updates(&mut self) -> UpdateFlags {
        let now = Instant::now();

        if now.duration_since(self.last_clock_update) >= self.clock_interval {
            self.pending.clock = true;
            self.last_clock_update = now;
        }

        if now.duration_since(self.last_weather_update) >= self.weather_interval {
            self.pending.weather = true;
            self.last_weather_update = now;
        }

        let flags = self.pending;
        self.pending.clear();
        flags
    }

    /// Force an immediate update of all components
    pub fn force_update_all(&mut self) {
        self.pending.set_all();
    }

    /// Force an immediate clock update
    pub fn force_clock_update(&mut self) {
        self.pending.clock = true;
    }

    /// Force an immediate weather update
    pub fn force_weather_update(&mut self) {
        self.pending.weather = true;
    }

    /// Get time until next update
    pub fn time_until_next_update(&self) -> Duration {
        let now = Instant::now();

        let clock_remaining = self.clock_interval
            .checked_sub(now.duration_since(self.last_clock_update))
            .unwrap_or(Duration::ZERO);

        let weather_remaining = self.weather_interval
            .checked_sub(now.duration_since(self.last_weather_update))
            .unwrap_or(Duration::ZERO);

        clock_remaining.min(weather_remaining)
    }
}

impl Default for UpdateScheduler {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(1),      // Clock: every second
            Duration::from_secs(600),    // Weather: every 10 minutes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_flags() {
        let mut flags = UpdateFlags::new();
        assert!(!flags.needs_redraw());

        flags.clock = true;
        assert!(flags.needs_redraw());

        flags.clear();
        assert!(!flags.needs_redraw());
    }

    #[test]
    fn test_update_scheduler_creation() {
        let scheduler = UpdateScheduler::default();
        assert_eq!(scheduler.clock_interval, Duration::from_secs(1));
    }

    #[test]
    fn test_force_update() {
        let mut scheduler = UpdateScheduler::default();
        scheduler.force_update_all();

        let flags = scheduler.check_updates();
        assert!(flags.clock);
        assert!(flags.weather);
    }

    #[test]
    fn test_check_updates_clears_pending() {
        let mut scheduler = UpdateScheduler::default();
        scheduler.force_clock_update();

        let flags1 = scheduler.check_updates();
        assert!(flags1.clock);

        // Should be cleared now (unless interval passed)
        let flags2 = scheduler.check_updates();
        // Clock might be true again if 1 second passed, but weather should be false
        assert!(!flags2.weather);
    }

    #[test]
    fn test_update_flags_set_all() {
        let mut flags = UpdateFlags::new();
        flags.set_all();

        assert!(flags.clock);
        assert!(flags.weather);
        assert!(flags.layout);
        assert!(flags.theme);
        assert!(flags.needs_redraw());
    }

    #[test]
    fn test_force_clock_update() {
        let mut scheduler = UpdateScheduler::default();
        scheduler.force_clock_update();

        let flags = scheduler.check_updates();
        assert!(flags.clock);
        // Weather should not be flagged unless interval passed
    }

    #[test]
    fn test_force_weather_update() {
        let mut scheduler = UpdateScheduler::default();
        scheduler.force_weather_update();

        let flags = scheduler.check_updates();
        assert!(flags.weather);
    }

    #[test]
    fn test_time_until_next_update() {
        let scheduler = UpdateScheduler::default();
        let time = scheduler.time_until_next_update();

        // Should be less than or equal to clock interval
        assert!(time <= Duration::from_secs(1));
    }
}
