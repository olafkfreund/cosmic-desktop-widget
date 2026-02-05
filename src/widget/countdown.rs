//! Countdown Timer widget
//!
//! This widget displays a countdown to a target date/time.

use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use tracing::debug;

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// Countdown widget showing time remaining until a target
pub struct CountdownWidget {
    label: String,
    target: DateTime<Local>,
    last_update: Instant,

    // Configuration
    show_days: bool,
    show_hours: bool,
    show_minutes: bool,
    show_seconds: bool,
}

impl CountdownWidget {
    /// Create a new Countdown widget
    pub fn new(
        label: &str,
        target: DateTime<Local>,
        show_days: bool,
        show_hours: bool,
        show_minutes: bool,
        show_seconds: bool,
    ) -> Self {
        Self {
            label: label.to_string(),
            target,
            last_update: Instant::now(),
            show_days,
            show_hours,
            show_minutes,
            show_seconds,
        }
    }

    /// Create from a date string (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS)
    pub fn from_date_string(
        label: &str,
        date_str: &str,
        show_days: bool,
        show_hours: bool,
        show_minutes: bool,
        show_seconds: bool,
    ) -> anyhow::Result<Self> {
        let target = Self::parse_datetime(date_str)?;
        Ok(Self::new(
            label,
            target,
            show_days,
            show_hours,
            show_minutes,
            show_seconds,
        ))
    }

    /// Parse a datetime string
    fn parse_datetime(date_str: &str) -> anyhow::Result<DateTime<Local>> {
        // Try parsing as full datetime first
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
            return Local
                .from_local_datetime(&dt)
                .single()
                .context("Invalid datetime");
        }

        // Try parsing as date only (assume midnight)
        if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let dt = d.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            return Local
                .from_local_datetime(&dt)
                .single()
                .context("Invalid date");
        }

        bail!(
            "Invalid date format '{}'. Use YYYY-MM-DD or YYYY-MM-DD HH:MM:SS",
            date_str
        )
    }

    /// Calculate remaining time
    fn remaining(&self) -> chrono::Duration {
        let now = Local::now();
        self.target - now
    }

    /// Format the countdown display
    pub fn display_string(&self) -> String {
        let remaining = self.remaining();

        // Check if countdown has passed
        if remaining < chrono::Duration::zero() {
            return format!("{}: Passed!", self.label);
        }

        let total_seconds = remaining.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        let mut parts = Vec::new();

        if self.show_days && days > 0 {
            parts.push(format!("{}d", days));
        }

        if self.show_hours && (hours > 0 || days > 0) {
            parts.push(format!("{}h", hours));
        }

        if self.show_minutes && (minutes > 0 || hours > 0 || days > 0) {
            parts.push(format!("{}m", minutes));
        }

        if self.show_seconds {
            parts.push(format!("{}s", seconds));
        }

        if parts.is_empty() {
            format!("{}: Now!", self.label)
        } else {
            format!("{}: {}", self.label, parts.join(" "))
        }
    }
}

impl Widget for CountdownWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "countdown",
            name: "Countdown",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Update every second for accurate countdown
        self.last_update = Instant::now();
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        if self.show_seconds {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(60)
        }
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for CountdownWidget
pub struct CountdownWidgetFactory;

impl DynWidgetFactory for CountdownWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "countdown"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let label = config
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Countdown");

        let target_date = config
            .get("target_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2025-12-31");

        let show_days = config
            .get("show_days")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_hours = config
            .get("show_hours")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_minutes = config
            .get("show_minutes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_seconds = config
            .get("show_seconds")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!(
            label = %label,
            target_date = %target_date,
            "Creating CountdownWidget"
        );

        CountdownWidget::from_date_string(
            label,
            target_date,
            show_days,
            show_hours,
            show_minutes,
            show_seconds,
        )
        .map(|w| Box::new(w) as Box<dyn Widget>)
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert(
            "label".to_string(),
            toml::Value::String("New Year".to_string()),
        );
        config.insert(
            "target_date".to_string(),
            toml::Value::String("2026-01-01".to_string()),
        );
        config.insert("show_days".to_string(), toml::Value::Boolean(true));
        config.insert("show_hours".to_string(), toml::Value::Boolean(true));
        config.insert("show_minutes".to_string(), toml::Value::Boolean(true));
        config.insert("show_seconds".to_string(), toml::Value::Boolean(false));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(target) = config.get("target_date") {
            let target_str = target.as_str().context("'target_date' must be a string")?;

            // Validate date format
            CountdownWidget::parse_datetime(target_str)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_countdown_creation() {
        let target = Local::now() + chrono::Duration::days(10);
        let widget = CountdownWidget::new("Test", target, true, true, true, true);
        assert_eq!(widget.info().id, "countdown");
    }

    #[test]
    fn test_countdown_display() {
        let target = Local::now() + chrono::Duration::days(1) + chrono::Duration::hours(2);
        let widget = CountdownWidget::new("Test", target, true, true, true, false);
        let display = widget.display_string();
        assert!(display.contains("Test:"));
        assert!(display.contains("d") || display.contains("h"));
    }

    #[test]
    fn test_countdown_past() {
        let target = Local::now() - chrono::Duration::days(1);
        let widget = CountdownWidget::new("Past Event", target, true, true, true, true);
        let display = widget.display_string();
        assert!(display.contains("Passed!"));
    }

    #[test]
    fn test_parse_date() {
        let result = CountdownWidget::parse_datetime("2025-12-31");
        assert!(result.is_ok());

        let result = CountdownWidget::parse_datetime("2025-12-31 23:59:59");
        assert!(result.is_ok());

        let result = CountdownWidget::parse_datetime("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_creation() {
        let factory = CountdownWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "countdown");
    }
}
