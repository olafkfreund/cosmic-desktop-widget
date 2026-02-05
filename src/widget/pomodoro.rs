//! Pomodoro Timer widget
//!
//! This widget implements a Pomodoro timer with work/break cycles.
//! It supports configurable durations and auto-transitions between states.

use std::time::{Duration, Instant};

use anyhow::Context;
use tracing::debug;

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// Pomodoro timer states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroState {
    /// Idle - waiting to start
    Idle,
    /// Working - focus time
    Working,
    /// Short break between work sessions
    ShortBreak,
    /// Long break after multiple work sessions
    LongBreak,
}

impl PomodoroState {
    /// Get the emoji icon for this state
    fn icon(&self) -> &'static str {
        match self {
            PomodoroState::Idle => "🍅",
            PomodoroState::Working => "🍅",
            PomodoroState::ShortBreak => "☕",
            PomodoroState::LongBreak => "☕",
        }
    }

    /// Get the display name for this state
    fn name(&self) -> &'static str {
        match self {
            PomodoroState::Idle => "Ready to start",
            PomodoroState::Working => "Working",
            PomodoroState::ShortBreak => "Break",
            PomodoroState::LongBreak => "Break",
        }
    }
}

/// Pomodoro timer widget
pub struct PomodoroWidget {
    /// Current state
    state: PomodoroState,
    /// Time when current state started
    state_start: Option<Instant>,
    /// Duration of work sessions (in seconds)
    work_duration: u64,
    /// Duration of short breaks (in seconds)
    short_break_duration: u64,
    /// Duration of long breaks (in seconds)
    long_break_duration: u64,
    /// Number of pomodoros until long break
    pomodoros_until_long_break: u32,
    /// Count of completed work sessions
    completed_pomodoros: u32,
    /// Whether to auto-start breaks
    auto_start_breaks: bool,
    /// Whether to auto-start work after breaks
    auto_start_work: bool,
    /// Last update time
    last_update: Instant,
}

impl PomodoroWidget {
    /// Create a new Pomodoro widget
    pub fn new(
        work_duration: u64,
        short_break_duration: u64,
        long_break_duration: u64,
        pomodoros_until_long_break: u32,
        auto_start_breaks: bool,
        auto_start_work: bool,
    ) -> Self {
        Self {
            state: PomodoroState::Idle,
            state_start: None,
            work_duration,
            short_break_duration,
            long_break_duration,
            pomodoros_until_long_break,
            completed_pomodoros: 0,
            auto_start_breaks,
            auto_start_work,
            last_update: Instant::now(),
        }
    }

    /// Get the duration for the current state in seconds
    fn current_duration(&self) -> u64 {
        match self.state {
            PomodoroState::Idle => 0,
            PomodoroState::Working => self.work_duration,
            PomodoroState::ShortBreak => self.short_break_duration,
            PomodoroState::LongBreak => self.long_break_duration,
        }
    }

    /// Get elapsed time in current state
    fn elapsed(&self) -> Duration {
        match self.state_start {
            Some(start) => start.elapsed(),
            None => Duration::from_secs(0),
        }
    }

    /// Get remaining time in current state
    fn remaining(&self) -> Duration {
        let duration = Duration::from_secs(self.current_duration());
        let elapsed = self.elapsed();

        if elapsed >= duration {
            Duration::from_secs(0)
        } else {
            duration - elapsed
        }
    }

    /// Check if current state has completed
    fn is_state_complete(&self) -> bool {
        if self.state == PomodoroState::Idle {
            return false;
        }

        self.elapsed() >= Duration::from_secs(self.current_duration())
    }

    /// Transition to the next state
    fn transition_to_next_state(&mut self) {
        let next_state = match self.state {
            PomodoroState::Idle => {
                // Start working
                debug!("Starting work session");
                PomodoroState::Working
            }
            PomodoroState::Working => {
                // Complete a pomodoro
                self.completed_pomodoros += 1;
                debug!(
                    completed = self.completed_pomodoros,
                    "Completed work session"
                );

                // Determine break type
                if self.completed_pomodoros % self.pomodoros_until_long_break == 0 {
                    debug!("Starting long break");
                    PomodoroState::LongBreak
                } else {
                    debug!("Starting short break");
                    PomodoroState::ShortBreak
                }
            }
            PomodoroState::ShortBreak | PomodoroState::LongBreak => {
                debug!("Break complete");
                PomodoroState::Idle
            }
        };

        self.state = next_state;

        // Auto-start logic
        let should_start = match next_state {
            PomodoroState::Working => self.auto_start_work,
            PomodoroState::ShortBreak | PomodoroState::LongBreak => self.auto_start_breaks,
            PomodoroState::Idle => false,
        };

        if should_start {
            self.state_start = Some(Instant::now());
            debug!(state = ?next_state, "Auto-started next state");
        } else {
            self.state_start = None;
            debug!(state = ?next_state, "Waiting for manual start");
        }
    }

    /// Start the current state (manual start)
    pub fn start(&mut self) {
        if self.state == PomodoroState::Idle {
            self.transition_to_next_state();
        }

        if self.state_start.is_none() {
            self.state_start = Some(Instant::now());
            debug!(state = ?self.state, "Manually started");
        }
    }

    /// Reset the timer
    pub fn reset(&mut self) {
        self.state = PomodoroState::Idle;
        self.state_start = None;
        self.completed_pomodoros = 0;
        debug!("Timer reset");
    }

    /// Format the display string
    pub fn display_string(&self) -> String {
        let icon = self.state.icon();

        match self.state {
            PomodoroState::Idle => {
                if self.completed_pomodoros == 0 {
                    format!("{} Ready to start", icon)
                } else {
                    format!(
                        "{} Ready to start ({}/{})",
                        icon,
                        self.completed_pomodoros % self.pomodoros_until_long_break,
                        self.pomodoros_until_long_break
                    )
                }
            }
            PomodoroState::Working | PomodoroState::ShortBreak | PomodoroState::LongBreak => {
                if self.state_start.is_none() {
                    // Waiting to start
                    format!("{} {} - Ready", icon, self.state.name())
                } else {
                    let remaining = self.remaining();
                    let total_secs = remaining.as_secs();
                    let minutes = total_secs / 60;
                    let seconds = total_secs % 60;

                    if self.state == PomodoroState::Working {
                        let current = self.completed_pomodoros % self.pomodoros_until_long_break;
                        format!(
                            "{} {}: {:02}:{:02} ({}/{})",
                            icon,
                            self.state.name(),
                            minutes,
                            seconds,
                            current + 1,
                            self.pomodoros_until_long_break
                        )
                    } else {
                        format!(
                            "{} {}: {:02}:{:02}",
                            icon,
                            self.state.name(),
                            minutes,
                            seconds
                        )
                    }
                }
            }
        }
    }
}

impl Widget for PomodoroWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "pomodoro",
            name: "Pomodoro Timer",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Check if current state is complete and transition if needed
        if self.is_state_complete() {
            self.transition_to_next_state();
        }

        self.last_update = Instant::now();
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        // Update every second for accurate countdown
        Duration::from_secs(1)
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for PomodoroWidget
pub struct PomodoroWidgetFactory;

impl DynWidgetFactory for PomodoroWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "pomodoro"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let work_duration = config
            .get("work_duration")
            .and_then(|v| v.as_integer())
            .unwrap_or(25) as u64
            * 60; // Convert minutes to seconds

        let short_break = config
            .get("short_break")
            .and_then(|v| v.as_integer())
            .unwrap_or(5) as u64
            * 60;

        let long_break = config
            .get("long_break")
            .and_then(|v| v.as_integer())
            .unwrap_or(15) as u64
            * 60;

        let pomodoros_until_long_break = config
            .get("pomodoros_until_long_break")
            .and_then(|v| v.as_integer())
            .unwrap_or(4) as u32;

        let auto_start_breaks = config
            .get("auto_start_breaks")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let auto_start_work = config
            .get("auto_start_work")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!(
            work_duration = work_duration / 60,
            short_break = short_break / 60,
            long_break = long_break / 60,
            pomodoros_until_long_break,
            "Creating PomodoroWidget"
        );

        Ok(Box::new(PomodoroWidget::new(
            work_duration,
            short_break,
            long_break,
            pomodoros_until_long_break,
            auto_start_breaks,
            auto_start_work,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("work_duration".to_string(), toml::Value::Integer(25));
        config.insert("short_break".to_string(), toml::Value::Integer(5));
        config.insert("long_break".to_string(), toml::Value::Integer(15));
        config.insert(
            "pomodoros_until_long_break".to_string(),
            toml::Value::Integer(4),
        );
        config.insert("auto_start_breaks".to_string(), toml::Value::Boolean(true));
        config.insert("auto_start_work".to_string(), toml::Value::Boolean(false));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(work) = config.get("work_duration") {
            let work_val = work
                .as_integer()
                .context("'work_duration' must be an integer")?;

            if work_val < 1 {
                anyhow::bail!("'work_duration' must be at least 1 minute");
            }
        }

        if let Some(short) = config.get("short_break") {
            let short_val = short
                .as_integer()
                .context("'short_break' must be an integer")?;

            if short_val < 1 {
                anyhow::bail!("'short_break' must be at least 1 minute");
            }
        }

        if let Some(long) = config.get("long_break") {
            let long_val = long
                .as_integer()
                .context("'long_break' must be an integer")?;

            if long_val < 1 {
                anyhow::bail!("'long_break' must be at least 1 minute");
            }
        }

        if let Some(count) = config.get("pomodoros_until_long_break") {
            let count_val = count
                .as_integer()
                .context("'pomodoros_until_long_break' must be an integer")?;

            if count_val < 1 {
                anyhow::bail!("'pomodoros_until_long_break' must be at least 1");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pomodoro_creation() {
        let widget = PomodoroWidget::new(25 * 60, 5 * 60, 15 * 60, 4, true, false);
        assert_eq!(widget.state, PomodoroState::Idle);
        assert_eq!(widget.completed_pomodoros, 0);
        assert_eq!(widget.info().id, "pomodoro");
    }

    #[test]
    fn test_pomodoro_start() {
        let mut widget = PomodoroWidget::new(25 * 60, 5 * 60, 15 * 60, 4, true, false);
        widget.start();
        assert_eq!(widget.state, PomodoroState::Working);
        assert!(widget.state_start.is_some());
    }

    #[test]
    fn test_pomodoro_display_idle() {
        let widget = PomodoroWidget::new(25 * 60, 5 * 60, 15 * 60, 4, true, false);
        let display = widget.display_string();
        assert!(display.contains("🍅"));
        assert!(display.contains("Ready to start"));
    }

    #[test]
    fn test_pomodoro_display_working() {
        let mut widget = PomodoroWidget::new(25 * 60, 5 * 60, 15 * 60, 4, true, false);
        widget.start();
        let display = widget.display_string();
        assert!(display.contains("🍅"));
        assert!(display.contains("Working"));
        assert!(display.contains("(1/4)"));
    }

    #[test]
    fn test_pomodoro_transition_to_short_break() {
        let mut widget = PomodoroWidget::new(1, 1, 1, 4, true, false);
        widget.start();
        assert_eq!(widget.state, PomodoroState::Working);

        // Wait for work to complete
        std::thread::sleep(Duration::from_secs(2));
        widget.update();

        assert_eq!(widget.state, PomodoroState::ShortBreak);
        assert_eq!(widget.completed_pomodoros, 1);
    }

    #[test]
    fn test_pomodoro_transition_to_long_break() {
        let mut widget = PomodoroWidget::new(1, 1, 1, 2, true, false);

        // Complete first work session
        widget.start();
        std::thread::sleep(Duration::from_secs(2));
        widget.update();
        assert_eq!(widget.state, PomodoroState::ShortBreak);

        // Complete short break
        std::thread::sleep(Duration::from_secs(2));
        widget.update();

        // Start second work session
        widget.start();
        std::thread::sleep(Duration::from_secs(2));
        widget.update();

        // Should be long break now (after 2nd pomodoro)
        assert_eq!(widget.state, PomodoroState::LongBreak);
        assert_eq!(widget.completed_pomodoros, 2);
    }

    #[test]
    fn test_pomodoro_auto_start_breaks() {
        let mut widget = PomodoroWidget::new(1, 1, 1, 4, true, false);
        widget.start();

        // Wait for work to complete
        std::thread::sleep(Duration::from_secs(2));
        widget.update();

        // Should auto-start break
        assert_eq!(widget.state, PomodoroState::ShortBreak);
        assert!(widget.state_start.is_some());
    }

    #[test]
    fn test_pomodoro_no_auto_start_work() {
        let mut widget = PomodoroWidget::new(1, 1, 1, 4, true, false);
        widget.start();

        // Complete work and break
        std::thread::sleep(Duration::from_secs(2));
        widget.update();
        std::thread::sleep(Duration::from_secs(2));
        widget.update();

        // Should be idle (not auto-start work)
        assert_eq!(widget.state, PomodoroState::Idle);
        assert!(widget.state_start.is_none());
    }

    #[test]
    fn test_pomodoro_reset() {
        let mut widget = PomodoroWidget::new(25 * 60, 5 * 60, 15 * 60, 4, true, false);
        widget.start();
        widget.completed_pomodoros = 3;

        widget.reset();

        assert_eq!(widget.state, PomodoroState::Idle);
        assert_eq!(widget.completed_pomodoros, 0);
        assert!(widget.state_start.is_none());
    }

    #[test]
    fn test_factory_creation() {
        let factory = PomodoroWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "pomodoro");
    }

    #[test]
    fn test_factory_custom_config() {
        let factory = PomodoroWidgetFactory;
        let mut config = toml::Table::new();
        config.insert("work_duration".to_string(), toml::Value::Integer(30));
        config.insert("short_break".to_string(), toml::Value::Integer(10));
        config.insert("auto_start_work".to_string(), toml::Value::Boolean(true));

        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "pomodoro");
    }

    #[test]
    fn test_factory_validation() {
        let factory = PomodoroWidgetFactory;

        // Valid config
        let mut valid = toml::Table::new();
        valid.insert("work_duration".to_string(), toml::Value::Integer(25));
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid work duration (too small)
        let mut invalid = toml::Table::new();
        invalid.insert("work_duration".to_string(), toml::Value::Integer(0));
        assert!(factory.validate_config(&invalid).is_err());
    }

    #[test]
    fn test_remaining_time() {
        let mut widget = PomodoroWidget::new(60, 30, 90, 4, true, false);
        widget.start();

        let remaining = widget.remaining();
        assert!(remaining.as_secs() <= 60);
        assert!(remaining.as_secs() > 55); // Allow small margin for test execution
    }
}
