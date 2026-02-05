//! Calendar/Agenda widget
//!
//! This widget displays upcoming events from ICS calendar files.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use icalendar::parser::read_calendar;
use tracing::{debug, warn};

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// Calendar event with time and title
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    /// Event start time
    pub start: DateTime<Local>,
    /// Event title/summary
    pub title: String,
    /// Whether this is an all-day event
    pub all_day: bool,
}

/// Calendar widget showing upcoming events
pub struct CalendarWidget {
    /// Paths to ICS calendar files
    calendar_files: Vec<PathBuf>,
    /// Maximum number of events to display
    max_events: usize,
    /// Show all-day events
    show_all_day: bool,
    /// Number of days ahead to show
    days_ahead: i64,
    /// Update interval in seconds
    update_interval: Duration,
    /// Cached events
    events: Vec<CalendarEvent>,
    /// Last update time
    last_update: Instant,
    /// Error message if any
    error_message: Option<String>,
}

impl CalendarWidget {
    /// Create a new Calendar widget
    pub fn new(
        calendar_files: Vec<PathBuf>,
        max_events: usize,
        show_all_day: bool,
        days_ahead: i64,
        update_interval: u64,
    ) -> Self {
        let mut widget = Self {
            calendar_files,
            max_events,
            show_all_day,
            days_ahead,
            update_interval: Duration::from_secs(update_interval),
            events: Vec::new(),
            last_update: Instant::now(),
            error_message: None,
        };

        // Load events on creation
        if let Err(e) = widget.load_events() {
            widget.error_message = Some(format!("Failed to load events: {}", e));
        }

        widget
    }

    /// Load events from all calendar files
    fn load_events(&mut self) -> Result<()> {
        let mut all_events = Vec::new();

        for calendar_file in &self.calendar_files {
            match self.parse_calendar(calendar_file) {
                Ok(mut events) => {
                    debug!(
                        file = %calendar_file.display(),
                        event_count = %events.len(),
                        "Loaded calendar events"
                    );
                    all_events.append(&mut events);
                }
                Err(e) => {
                    warn!(
                        file = %calendar_file.display(),
                        error = %e,
                        "Failed to parse calendar file"
                    );
                    // Continue with other files instead of failing completely
                }
            }
        }

        // Filter events to show only upcoming ones within the time range
        let now = Local::now();
        let end_date = now + chrono::Duration::days(self.days_ahead);

        all_events.retain(|event| {
            // Check if event is in our time range
            if event.start < now || event.start > end_date {
                return false;
            }

            // Filter all-day events if needed
            if !self.show_all_day && event.all_day {
                return false;
            }

            true
        });

        // Sort by start time
        all_events.sort_by_key(|e| e.start);

        // Limit to max events
        all_events.truncate(self.max_events);

        self.events = all_events;
        self.error_message = None;

        Ok(())
    }

    /// Parse a single ICS calendar file
    fn parse_calendar(&self, path: &Path) -> Result<Vec<CalendarEvent>> {
        // Check if file exists
        if !path.exists() {
            bail!("Calendar file does not exist: {}", path.display());
        }

        // Read file content
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read calendar file: {}", path.display()))?;

        // Parse ICS content
        let calendar = read_calendar(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse ICS file: {:?}", e))?;

        let mut events = Vec::new();

        // Extract events from calendar
        for component in calendar.components {
            if component.name != "VEVENT" {
                continue;
            }

            // Extract event properties
            let mut title = String::from("Untitled Event");
            let mut start_str: Option<String> = None;
            let mut all_day = false;

            for property in component.properties {
                match property.name.as_str() {
                    "SUMMARY" => {
                        title = property.val.to_string();
                    }
                    "DTSTART" => {
                        let val = property.val.to_string();
                        start_str = Some(val.clone());

                        // Check if this is an all-day event (DATE format, no time)
                        // All-day events are typically in format YYYYMMDD (8 chars)
                        // Events with time are YYYYMMDDTHHMMSS (15+ chars)
                        if val.len() == 8 {
                            all_day = true;
                        }
                    }
                    _ => {}
                }
            }

            // Parse start time
            if let Some(start) = start_str {
                if let Some(dt) = self.parse_ics_datetime(&start, all_day) {
                    events.push(CalendarEvent {
                        start: dt,
                        title,
                        all_day,
                    });
                }
            }
        }

        Ok(events)
    }

    /// Parse ICS datetime format
    fn parse_ics_datetime(&self, datetime_str: &str, all_day: bool) -> Option<DateTime<Local>> {
        // Remove any timezone suffix (e.g., "Z" for UTC)
        let datetime_str = datetime_str.trim_end_matches('Z');

        // ICS datetime formats:
        // - YYYYMMDD (all-day)
        // - YYYYMMDDTHHMMSS (local time)
        // - YYYYMMDDTHHMMSSZ (UTC)

        if all_day && datetime_str.len() == 8 {
            // Parse date only (YYYYMMDD)
            if let Ok(year) = datetime_str[0..4].parse::<i32>() {
                if let Ok(month) = datetime_str[4..6].parse::<u32>() {
                    if let Ok(day) = datetime_str[6..8].parse::<u32>() {
                        // Set to start of day
                        return Local.with_ymd_and_hms(year, month, day, 0, 0, 0).single();
                    }
                }
            }
        } else if datetime_str.len() >= 15 {
            // Parse datetime (YYYYMMDDTHHMMSS)
            let date_part = &datetime_str[0..8];
            let time_part = &datetime_str[9..15];

            if let Ok(year) = date_part[0..4].parse::<i32>() {
                if let Ok(month) = date_part[4..6].parse::<u32>() {
                    if let Ok(day) = date_part[6..8].parse::<u32>() {
                        if let Ok(hour) = time_part[0..2].parse::<u32>() {
                            if let Ok(minute) = time_part[2..4].parse::<u32>() {
                                if let Ok(second) = time_part[4..6].parse::<u32>() {
                                    return Local
                                        .with_ymd_and_hms(year, month, day, hour, minute, second)
                                        .single();
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Format events for display
    pub fn display_string(&self) -> String {
        if let Some(ref err) = self.error_message {
            return format!("Error: {}", err);
        }

        if self.events.is_empty() {
            return "No upcoming events".to_string();
        }

        let now = Local::now();
        let today_date = now.date_naive();
        let tomorrow_date = (now + chrono::Duration::days(1)).date_naive();

        // Group events by day
        let mut today_events = Vec::new();
        let mut tomorrow_events = Vec::new();
        let mut future_events = Vec::new();

        for event in &self.events {
            let event_date = event.start.date_naive();
            if event_date == today_date {
                today_events.push(event);
            } else if event_date == tomorrow_date {
                tomorrow_events.push(event);
            } else {
                future_events.push(event);
            }
        }

        let mut lines = Vec::new();

        // Format today's events
        if !today_events.is_empty() {
            let events_str: Vec<String> =
                today_events.iter().map(|e| self.format_event(e)).collect();
            lines.push(format!("Today: {}", events_str.join(" | ")));
        }

        // Format tomorrow's events
        if !tomorrow_events.is_empty() {
            let events_str: Vec<String> = tomorrow_events
                .iter()
                .map(|e| self.format_event(e))
                .collect();
            lines.push(format!("Tomorrow: {}", events_str.join(" | ")));
        }

        // Format future events (with date)
        for event in future_events {
            let date_str = event.start.format("%a %b %d").to_string();
            lines.push(format!("{}: {}", date_str, self.format_event(event)));
        }

        lines.join("\n")
    }

    /// Format a single event
    fn format_event(&self, event: &CalendarEvent) -> String {
        if event.all_day {
            event.title.clone()
        } else {
            format!("{} {}", event.start.format("%H:%M"), event.title)
        }
    }
}

impl Widget for CalendarWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "calendar",
            name: "Calendar",
            preferred_height: 60.0,
            min_height: 40.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Reload events if update interval has passed
        if self.last_update.elapsed() >= self.update_interval {
            if let Err(e) = self.load_events() {
                self.error_message = Some(format!("Failed to update events: {}", e));
            }
            self.last_update = Instant::now();
        }
    }

    fn content(&self) -> WidgetContent {
        let text = self.display_string();

        // Use multiline if we have multiple days
        if text.contains('\n') {
            let lines: Vec<(String, FontSize)> = text
                .lines()
                .map(|line| (line.to_string(), FontSize::Medium))
                .collect();

            WidgetContent::MultiLine { lines }
        } else {
            WidgetContent::Text {
                text,
                size: FontSize::Medium,
            }
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        true // Always ready, even if no events
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for CalendarWidget
pub struct CalendarWidgetFactory;

impl DynWidgetFactory for CalendarWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "calendar"
    }

    fn create(&self, config: &toml::Table) -> Result<Box<dyn Widget>> {
        // Parse calendar files array
        let calendar_files = if let Some(files) = config.get("calendar_files") {
            match files.as_array() {
                Some(arr) => {
                    let mut paths = Vec::new();
                    for file_val in arr {
                        if let Some(file_str) = file_val.as_str() {
                            // Expand ~ to home directory
                            let path = if file_str.starts_with("~/") {
                                if let Some(home) = dirs::home_dir() {
                                    home.join(&file_str[2..])
                                } else {
                                    PathBuf::from(file_str)
                                }
                            } else {
                                PathBuf::from(file_str)
                            };
                            paths.push(path);
                        }
                    }
                    paths
                }
                None => bail!("'calendar_files' must be an array of strings"),
            }
        } else {
            // Default: try common calendar locations
            vec![dirs::home_dir()
                .map(|h| h.join(".local/share/calendar/calendar.ics"))
                .unwrap_or_else(|| PathBuf::from("calendar.ics"))]
        };

        let max_events = config
            .get("max_events")
            .and_then(|v| v.as_integer())
            .unwrap_or(5) as usize;

        let show_all_day = config
            .get("show_all_day")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let days_ahead = config
            .get("days_ahead")
            .and_then(|v| v.as_integer())
            .unwrap_or(2) as i64;

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(300) as u64;

        debug!(
            calendar_files = ?calendar_files,
            max_events = %max_events,
            show_all_day = %show_all_day,
            days_ahead = %days_ahead,
            update_interval = %update_interval,
            "Creating CalendarWidget"
        );

        Ok(Box::new(CalendarWidget::new(
            calendar_files,
            max_events,
            show_all_day,
            days_ahead,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();

        // Default calendar files
        let default_path = dirs::home_dir()
            .map(|h| format!("{}/.local/share/calendar/calendar.ics", h.display()))
            .unwrap_or_else(|| "calendar.ics".to_string());

        config.insert(
            "calendar_files".to_string(),
            toml::Value::Array(vec![toml::Value::String(default_path)]),
        );
        config.insert("max_events".to_string(), toml::Value::Integer(5));
        config.insert("show_all_day".to_string(), toml::Value::Boolean(true));
        config.insert("days_ahead".to_string(), toml::Value::Integer(2));
        config.insert("update_interval".to_string(), toml::Value::Integer(300));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> Result<()> {
        if let Some(files) = config.get("calendar_files") {
            if !files.is_array() {
                bail!("'calendar_files' must be an array of strings");
            }
        }

        if let Some(max_events) = config.get("max_events") {
            let val = max_events
                .as_integer()
                .context("'max_events' must be an integer")?;

            if val < 1 {
                bail!("'max_events' must be at least 1");
            }
        }

        if let Some(days_ahead) = config.get("days_ahead") {
            let val = days_ahead
                .as_integer()
                .context("'days_ahead' must be an integer")?;

            if val < 0 {
                bail!("'days_ahead' must be non-negative");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use chrono::Timelike;

    /// Create a temporary ICS file for testing
    fn create_test_ics(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_calendar_creation() {
        let widget = CalendarWidget::new(vec![PathBuf::from("nonexistent.ics")], 5, true, 2, 300);
        assert_eq!(widget.info().id, "calendar");
    }

    #[test]
    fn test_parse_ics_datetime() {
        let widget = CalendarWidget::new(vec![], 5, true, 2, 300);

        // Test all-day event
        let dt = widget.parse_ics_datetime("20250206", true);
        assert!(dt.is_some());

        // Test datetime event
        let dt = widget.parse_ics_datetime("20250206T143000", false);
        assert!(dt.is_some());
        if let Some(dt) = dt {
            assert_eq!(dt.hour(), 14);
            assert_eq!(dt.minute(), 30);
        }
    }

    #[test]
    fn test_parse_calendar_with_events() {
        let ics_content = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test1@example.com
DTSTART:20250206T100000
SUMMARY:Team Meeting
END:VEVENT
BEGIN:VEVENT
UID:test2@example.com
DTSTART:20250206T140000
SUMMARY:Doctor Appointment
END:VEVENT
END:VCALENDAR"#;

        let temp_file = create_test_ics(ics_content);
        let widget = CalendarWidget::new(vec![], 5, true, 2, 300);

        let events = widget.parse_calendar(temp_file.path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Team Meeting");
        assert_eq!(events[1].title, "Doctor Appointment");
    }

    #[test]
    fn test_format_event() {
        let widget = CalendarWidget::new(vec![], 5, true, 2, 300);

        let now = Local::now();
        let event = CalendarEvent {
            start: now,
            title: "Test Event".to_string(),
            all_day: false,
        };

        let formatted = widget.format_event(&event);
        assert!(formatted.contains("Test Event"));
        assert!(formatted.contains(":")); // Should have time
    }

    #[test]
    fn test_format_all_day_event() {
        let widget = CalendarWidget::new(vec![], 5, true, 2, 300);

        let now = Local::now();
        let event = CalendarEvent {
            start: now,
            title: "All Day Event".to_string(),
            all_day: true,
        };

        let formatted = widget.format_event(&event);
        assert_eq!(formatted, "All Day Event");
    }

    #[test]
    fn test_display_string_no_events() {
        let widget = CalendarWidget::new(vec![], 5, true, 2, 300);
        let display = widget.display_string();
        assert!(display.contains("No upcoming events"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = CalendarWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "calendar");
    }

    #[test]
    fn test_factory_validation() {
        let factory = CalendarWidgetFactory;

        // Valid config
        let valid = factory.default_config();
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid max_events
        let mut invalid = toml::Table::new();
        invalid.insert("max_events".to_string(), toml::Value::Integer(0));
        assert!(factory.validate_config(&invalid).is_err());

        // Invalid days_ahead
        let mut invalid = toml::Table::new();
        invalid.insert("days_ahead".to_string(), toml::Value::Integer(-1));
        assert!(factory.validate_config(&invalid).is_err());
    }
}
