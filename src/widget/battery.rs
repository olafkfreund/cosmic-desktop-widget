//! Battery widget displaying battery status, percentage, and time remaining
//!
//! This widget reads battery information from /sys/class/power_supply/ and displays:
//! - Battery percentage
//! - Charging/discharging status
//! - Time remaining (optional)
//! - Supports multiple batteries
//! - Gracefully handles systems without batteries

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// Battery status information
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    /// Battery percentage (0-100)
    pub percentage: u8,
    /// Current status (Charging, Discharging, Full, etc.)
    pub status: BatteryStatus,
    /// Energy now in microwatt-hours (optional)
    pub energy_now: Option<u64>,
    /// Energy full in microwatt-hours (optional)
    pub energy_full: Option<u64>,
    /// Power now in microwatts (optional, for time calculation)
    pub power_now: Option<u64>,
}

/// Battery charging status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    /// Battery is charging
    Charging,
    /// Battery is discharging
    Discharging,
    /// Battery is full
    Full,
    /// Battery is not charging (plugged in but not charging)
    NotCharging,
    /// Unknown status
    Unknown,
}

impl BatteryStatus {
    /// Parse status from string
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "Charging" => Self::Charging,
            "Discharging" => Self::Discharging,
            "Full" => Self::Full,
            "Not charging" => Self::NotCharging,
            _ => Self::Unknown,
        }
    }

    /// Get display string for status
    fn display_str(&self) -> &'static str {
        match self {
            Self::Charging => "Charging",
            Self::Discharging => "Discharging",
            Self::Full => "Full",
            Self::NotCharging => "Not charging",
            Self::Unknown => "Unknown",
        }
    }

    /// Get emoji icon for status
    fn icon(&self) -> &'static str {
        match self {
            Self::Charging => "🔌",
            Self::Discharging => "🔋",
            Self::Full => "🔌",
            Self::NotCharging => "🔌",
            Self::Unknown => "🔋",
        }
    }
}

/// Battery widget displaying battery status
pub struct BatteryWidget {
    battery_info: Option<BatteryInfo>,
    last_update: Instant,
    update_interval: Duration,
    battery_path: Option<PathBuf>,

    // Configuration
    show_percentage: bool,
    show_status: bool,
    show_time_remaining: bool,

    // Error state
    error_message: Option<String>,
}

impl BatteryWidget {
    /// Create a new Battery widget
    pub fn new(
        show_percentage: bool,
        show_status: bool,
        show_time_remaining: bool,
        battery_path: Option<String>,
        update_interval: u64,
    ) -> Self {
        let battery_path = battery_path.map(PathBuf::from);

        let mut widget = Self {
            battery_info: None,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(update_interval),
            battery_path,
            show_percentage,
            show_status,
            show_time_remaining,
            error_message: None,
        };

        // Initial update
        widget.update_battery_info();

        widget
    }

    /// Find the first available battery
    fn find_battery() -> Option<PathBuf> {
        let power_supply = PathBuf::from("/sys/class/power_supply");

        if !power_supply.exists() {
            debug!("/sys/class/power_supply does not exist");
            return None;
        }

        // Look for BAT0, BAT1, etc.
        for i in 0..10 {
            let bat_path = power_supply.join(format!("BAT{}", i));
            if bat_path.exists() {
                debug!(path = ?bat_path, "Found battery");
                return Some(bat_path);
            }
        }

        // Try to find any directory with "battery" in the name
        if let Ok(entries) = fs::read_dir(&power_supply) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase().contains("bat") {
                        debug!(path = ?path, "Found battery");
                        return Some(path);
                    }
                }
            }
        }

        debug!("No battery found in /sys/class/power_supply");
        None
    }

    /// Read battery information from sysfs
    fn read_battery_info(battery_path: &PathBuf) -> Result<BatteryInfo, String> {
        // Read capacity (percentage)
        let capacity_path = battery_path.join("capacity");
        let percentage = fs::read_to_string(&capacity_path)
            .map_err(|e| format!("Failed to read capacity: {}", e))?
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Failed to parse capacity: {}", e))?
            .min(100); // Clamp to 100

        // Read status
        let status_path = battery_path.join("status");
        let status_str = fs::read_to_string(&status_path)
            .map_err(|e| format!("Failed to read status: {}", e))?;
        let status = BatteryStatus::from_str(&status_str);

        // Read energy information (optional, for time remaining)
        let energy_now = Self::read_u64(battery_path, "energy_now");
        let energy_full = Self::read_u64(battery_path, "energy_full");
        let power_now = Self::read_u64(battery_path, "power_now");

        Ok(BatteryInfo {
            percentage,
            status,
            energy_now,
            energy_full,
            power_now,
        })
    }

    /// Helper to read u64 value from sysfs file
    fn read_u64(base_path: &PathBuf, filename: &str) -> Option<u64> {
        let path = base_path.join(filename);
        fs::read_to_string(&path).ok()?.trim().parse::<u64>().ok()
    }

    /// Update battery information
    fn update_battery_info(&mut self) {
        // Find battery if not already set
        if self.battery_path.is_none() {
            self.battery_path = Self::find_battery();
        }

        // If still no battery, set error and return
        let Some(ref battery_path) = self.battery_path else {
            self.error_message = Some("No battery found".to_string());
            self.battery_info = None;
            return;
        };

        // Read battery information
        match Self::read_battery_info(battery_path) {
            Ok(info) => {
                debug!(
                    percentage = %info.percentage,
                    status = ?info.status,
                    "Battery info updated"
                );
                self.battery_info = Some(info);
                self.error_message = None;
            }
            Err(e) => {
                warn!(error = %e, "Failed to read battery info");
                self.error_message = Some(e);
                // Keep old info if available
            }
        }
    }

    /// Calculate time remaining in minutes
    fn calculate_time_remaining(&self) -> Option<u64> {
        let info = self.battery_info.as_ref()?;
        let power_now = info.power_now?;

        // Avoid division by zero
        if power_now == 0 {
            return None;
        }

        let minutes = match info.status {
            BatteryStatus::Discharging => {
                // Time until empty
                let energy_now = info.energy_now?;
                // energy is in µWh, power is in µW
                // time (hours) = energy / power
                let hours = energy_now as f64 / power_now as f64;
                (hours * 60.0) as u64
            }
            BatteryStatus::Charging => {
                // Time until full
                let energy_now = info.energy_now?;
                let energy_full = info.energy_full?;
                let energy_to_charge = energy_full.saturating_sub(energy_now);
                let hours = energy_to_charge as f64 / power_now as f64;
                (hours * 60.0) as u64
            }
            _ => return None,
        };

        Some(minutes)
    }

    /// Format time remaining as string
    fn format_time_remaining(minutes: u64) -> String {
        let hours = minutes / 60;
        let mins = minutes % 60;

        if hours > 0 {
            format!("{}h {:02}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    /// Generate display string
    pub fn display_string(&self) -> String {
        // If there's an error and no data, show error
        if self.battery_info.is_none() {
            return self
                .error_message
                .as_ref()
                .map(|e| format!("Battery: {}", e))
                .unwrap_or_else(|| "No battery".to_string());
        }

        let info = self.battery_info.as_ref().unwrap();
        let mut parts = Vec::new();

        // Icon
        parts.push(info.status.icon().to_string());

        // Percentage
        if self.show_percentage {
            parts.push(format!("{}%", info.percentage));
        }

        // Status
        if self.show_status {
            parts.push(info.status.display_str().to_string());
        }

        // Time remaining
        if self.show_time_remaining {
            if let Some(minutes) = self.calculate_time_remaining() {
                parts.push(format!("({})", Self::format_time_remaining(minutes)));
            }
        }

        // Show error indicator if there's an error but we have old data
        if self.error_message.is_some() {
            parts.push("⚠".to_string());
        }

        parts.join(" ")
    }
}

impl Widget for BatteryWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "battery",
            name: "Battery",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() < self.update_interval {
            return;
        }

        self.update_battery_info();
        self.last_update = Instant::now();
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        // Ready if we have info or an error message
        self.battery_info.is_some() || self.error_message.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

impl Default for BatteryWidget {
    fn default() -> Self {
        Self::new(true, true, false, None, 30)
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for BatteryWidget
pub struct BatteryWidgetFactory;

impl DynWidgetFactory for BatteryWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "battery"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let show_percentage = config
            .get("show_percentage")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_status = config
            .get("show_status")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_time_remaining = config
            .get("show_time_remaining")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let battery_path = config
            .get("battery_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(30) as u64;

        debug!(
            show_percentage = %show_percentage,
            show_status = %show_status,
            show_time_remaining = %show_time_remaining,
            battery_path = ?battery_path,
            update_interval = %update_interval,
            "Creating BatteryWidget"
        );

        Ok(Box::new(BatteryWidget::new(
            show_percentage,
            show_status,
            show_time_remaining,
            battery_path,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("show_percentage".to_string(), toml::Value::Boolean(true));
        config.insert("show_status".to_string(), toml::Value::Boolean(true));
        config.insert(
            "show_time_remaining".to_string(),
            toml::Value::Boolean(false),
        );
        config.insert("update_interval".to_string(), toml::Value::Integer(30));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        if let Some(interval) = config.get("update_interval") {
            let interval_val = interval
                .as_integer()
                .ok_or_else(|| anyhow::anyhow!("'update_interval' must be an integer"))?;

            if interval_val < 1 {
                anyhow::bail!("'update_interval' must be at least 1 second");
            }
        }

        if let Some(path) = config.get("battery_path") {
            path.as_str()
                .ok_or_else(|| anyhow::anyhow!("'battery_path' must be a string"))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_status_parsing() {
        assert_eq!(BatteryStatus::from_str("Charging"), BatteryStatus::Charging);
        assert_eq!(
            BatteryStatus::from_str("Discharging"),
            BatteryStatus::Discharging
        );
        assert_eq!(BatteryStatus::from_str("Full"), BatteryStatus::Full);
        assert_eq!(
            BatteryStatus::from_str("Not charging"),
            BatteryStatus::NotCharging
        );
        assert_eq!(BatteryStatus::from_str("Unknown"), BatteryStatus::Unknown);
    }

    #[test]
    fn test_battery_status_display() {
        assert_eq!(BatteryStatus::Charging.display_str(), "Charging");
        assert_eq!(BatteryStatus::Discharging.display_str(), "Discharging");
        assert_eq!(BatteryStatus::Full.display_str(), "Full");
    }

    #[test]
    fn test_battery_status_icon() {
        assert_eq!(BatteryStatus::Charging.icon(), "🔌");
        assert_eq!(BatteryStatus::Discharging.icon(), "🔋");
        assert_eq!(BatteryStatus::Full.icon(), "🔌");
    }

    #[test]
    fn test_format_time_remaining() {
        assert_eq!(BatteryWidget::format_time_remaining(45), "45m");
        assert_eq!(BatteryWidget::format_time_remaining(90), "1h 30m");
        assert_eq!(BatteryWidget::format_time_remaining(125), "2h 05m");
        assert_eq!(BatteryWidget::format_time_remaining(180), "3h 00m");
    }

    #[test]
    fn test_battery_widget_creation() {
        let widget = BatteryWidget::default();
        assert_eq!(widget.info().id, "battery");
    }

    #[test]
    fn test_battery_widget_no_battery() {
        // Create widget with non-existent path
        let widget = BatteryWidget::new(
            true,
            true,
            false,
            Some("/non/existent/path".to_string()),
            30,
        );

        // Should have error message
        assert!(widget.error_message.is_some());

        // Display string should indicate no battery
        let display = widget.display_string();
        assert!(display.contains("Battery:") || display.contains("No battery"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = BatteryWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "battery");
    }

    #[test]
    fn test_factory_validation() {
        let factory = BatteryWidgetFactory;

        // Valid config
        let valid = factory.default_config();
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid update_interval
        let mut invalid = toml::Table::new();
        invalid.insert("update_interval".to_string(), toml::Value::Integer(0));
        assert!(factory.validate_config(&invalid).is_err());
    }

    #[test]
    fn test_time_calculation() {
        let widget = BatteryWidget {
            battery_info: Some(BatteryInfo {
                percentage: 50,
                status: BatteryStatus::Discharging,
                energy_now: Some(50_000_000),   // 50 Wh
                energy_full: Some(100_000_000), // 100 Wh
                power_now: Some(10_000_000),    // 10 W
            }),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(30),
            battery_path: None,
            show_percentage: true,
            show_status: true,
            show_time_remaining: true,
            error_message: None,
        };

        // Should calculate time remaining
        let time = widget.calculate_time_remaining();
        assert!(time.is_some());

        // 50 Wh / 10 W = 5 hours = 300 minutes
        assert_eq!(time.unwrap(), 300);
    }
}
