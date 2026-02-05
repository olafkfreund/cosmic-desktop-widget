//! Widget implementations for COSMIC Desktop Widget
//!
//! This module provides the core widget types and the extensibility framework
//! for creating custom widgets.
//!
//! # Available Widgets
//!
//! - [`ClockWidget`] - Displays current time with configurable format
//! - [`WeatherWidget`] - Shows weather data from OpenWeatherMap API
//! - [`SystemMonitorWidget`] - CPU, RAM, and disk usage (requires `sysinfo` feature)
//! - [`CountdownWidget`] - Countdown to a target date/time
//! - [`CryptoWidget`] - Cryptocurrency prices from CoinGecko API
//! - [`CalendarWidget`] - Upcoming events from ICS calendar files
//! - [`PomodoroWidget`] - Pomodoro timer with work/break cycles
//! - [`QuotesWidget`] - Inspirational quotes display
//! - [`BatteryWidget`] - Battery status, percentage, and time remaining
//! - [`StocksWidget`] - Real-time stock prices from Yahoo Finance
//! - [`NewsWidget`] - News headlines from RSS feeds with rotation
//!
//! # Creating Custom Widgets
//!
//! To create a new widget, implement the [`Widget`] trait from the [`traits`] module.
//! See the [Widget Development Guide](../../docs/WIDGETS.md) for details.
//!
//! # Widget Registry
//!
//! The [`registry`] module provides dynamic widget creation from configuration.
//! Use [`WidgetRegistry::with_builtins()`](registry::WidgetRegistry::with_builtins)
//! to get a registry with all built-in widgets registered.

pub mod registry;
pub mod traits;

// New widgets
pub mod battery;
pub mod calendar;
pub mod countdown;
pub mod crypto;
pub mod mpris;
pub mod news;
pub mod pomodoro;
pub mod quotes;
pub mod stocks;
pub mod system_monitor;

pub use battery::BatteryWidget;
pub use calendar::CalendarWidget;
pub use countdown::CountdownWidget;
pub use crypto::{CryptoPrice, CryptoWidget};
pub use mpris::{MprisConfig, MprisWidget};
pub use news::{Headline, NewsWidget};
pub use pomodoro::{PomodoroState, PomodoroWidget};
pub use quotes::{Quote, QuotesWidget};
pub use registry::{DynWidgetFactory, WidgetInstance, WidgetRegistry};
pub use stocks::{StockData, StocksWidget};
pub use system_monitor::SystemMonitorWidget;
pub use traits::{
    FontSize, MouseButton, ProgressBar, ProgressColor, ScrollDirection, TextSegment, Widget,
    WidgetAction, WidgetConfig, WidgetContent, WidgetFactory, WidgetInfo,
};

use crate::error::{WeatherError, WeatherResult};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Clock widget displaying current time
///
/// Performance optimized:
/// - Caches formatted time and date strings
/// - Only updates when the second actually changes
/// - Provides borrowed string access to avoid allocations
pub struct ClockWidget {
    /// Cached formatted time string
    current_time: String,
    /// Cached date string (updated at midnight)
    current_date: String,
    /// Last update timestamp
    last_update: std::time::Instant,
    /// Last second value (0-59) to detect changes
    last_second: u32,
    /// Time format ("12h" or "24h")
    format: String,
    /// Whether to show seconds
    show_seconds: bool,
    /// Whether to show date
    show_date: bool,
    /// Whether content changed on last update
    changed: bool,
}

impl ClockWidget {
    pub fn new(format: &str, show_seconds: bool, show_date: bool) -> Self {
        use chrono::Timelike;
        let format_str = format.to_string();
        let now = Local::now();
        Self {
            current_time: Self::format_time_internal(&format_str, show_seconds),
            current_date: now.format("%A, %B %d, %Y").to_string(),
            last_update: std::time::Instant::now(),
            last_second: now.second(),
            format: format_str,
            show_seconds,
            show_date,
            changed: true, // First frame is always "changed"
        }
    }

    /// Update the clock if the second has changed
    pub fn update(&mut self) {
        use chrono::Timelike;
        let now = Local::now();
        let current_second = now.second();

        // Only update if the second changed
        if current_second != self.last_second {
            self.current_time = Self::format_time_internal(&self.format, self.show_seconds);
            self.last_second = current_second;
            self.last_update = std::time::Instant::now();
            self.changed = true;

            // Update date at midnight
            if now.hour() == 0 && now.minute() == 0 && current_second == 0 {
                self.current_date = now.format("%A, %B %d, %Y").to_string();
            }

            debug!(time = %self.current_time, "Clock updated");
        } else {
            self.changed = false;
        }
    }

    /// Get the current time string (clones the cached value)
    #[inline]
    pub fn time_string(&self) -> String {
        self.current_time.clone()
    }

    /// Get a reference to the current time string (no allocation)
    #[inline]
    pub fn time_str(&self) -> &str {
        &self.current_time
    }

    /// Get the current date string (clones the cached value)
    #[inline]
    pub fn date_string(&self) -> String {
        self.current_date.clone()
    }

    /// Get a reference to the current date string (no allocation)
    #[inline]
    pub fn date_str(&self) -> &str {
        &self.current_date
    }

    pub fn date_time_string(&self) -> String {
        if self.show_date {
            format!("{} - {}", self.current_date, self.current_time)
        } else {
            self.current_time.clone()
        }
    }

    /// Check if the clock display changed on the last update
    #[inline]
    pub fn has_changed(&self) -> bool {
        self.changed
    }

    fn format_time_internal(format: &str, show_seconds: bool) -> String {
        let now = Local::now();
        match (format, show_seconds) {
            ("12h", true) => now.format("%I:%M:%S %p").to_string(),
            ("12h", false) => now.format("%I:%M %p").to_string(),
            ("24h", true) | (_, true) => now.format("%H:%M:%S").to_string(),
            ("24h", false) | (_, false) => now.format("%H:%M").to_string(),
        }
    }
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self::new("24h", true, false)
    }
}

// Implement the Widget trait for ClockWidget
impl Widget for ClockWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "clock",
            name: "Clock",
            preferred_height: 80.0,
            min_height: 40.0,
            expand: true,
        }
    }

    fn update(&mut self) {
        // Delegate to existing update method
        ClockWidget::update(self);
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.time_string(),
            size: FontSize::Large,
        }
    }

    fn update_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

/// Weather widget displaying current weather conditions
pub struct WeatherWidget {
    city: String,
    api_key: String,
    data: Option<WeatherData>,
    last_update: std::time::Instant,
    update_interval: std::time::Duration,
    temperature_unit: String,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub temperature: f32,
    pub condition: String,
    pub humidity: u32,
    pub wind_speed: f32,
}

impl WeatherWidget {
    pub fn new(city: &str, api_key: &str, temperature_unit: &str, update_interval: u64) -> Self {
        Self {
            city: city.to_string(),
            api_key: api_key.to_string(),
            data: None,
            last_update: std::time::Instant::now(),
            update_interval: std::time::Duration::from_secs(update_interval),
            temperature_unit: temperature_unit.to_string(),
            error_message: None,
        }
    }

    /// Map weather condition to icon name
    pub fn condition_to_icon(condition: &str) -> &'static str {
        match condition.to_lowercase().as_str() {
            "clear" | "sunny" => "weather-clear",
            "clouds" | "cloudy" | "overcast" => "weather-clouds",
            "rain" | "drizzle" | "showers" => "weather-rain",
            "snow" | "sleet" | "flurries" => "weather-snow",
            "thunderstorm" | "storm" => "weather-storm",
            _ => "weather-clouds", // Default to clouds for unknown conditions
        }
    }

    pub fn update(&mut self) {
        // This method is now a no-op since weather fetching happens in the background thread
        // The actual updates come through set_data() and set_error()
    }

    /// Set weather data from successful API fetch
    pub fn set_data(&mut self, data: WeatherData) {
        debug!(
            temp = %data.temperature,
            condition = %data.condition,
            humidity = %data.humidity,
            "Weather data updated"
        );
        self.data = Some(data);
        self.last_update = std::time::Instant::now();
        self.error_message = None; // Clear any previous errors
    }

    /// Set error message from failed API fetch
    pub fn set_error(&mut self, error: String) {
        warn!(error = %error, "Weather fetch error");
        self.error_message = Some(error);
        // Keep old data if available
    }

    pub fn display_string(&self) -> Option<String> {
        // If there's an error and no data, show error
        if self.data.is_none() && self.error_message.is_some() {
            return self.error_message.as_ref().map(|e| format!("Error: {}", e));
        }

        self.data.as_ref().map(|data| {
            let (temp, unit) = match self.temperature_unit.as_str() {
                "fahrenheit" => ((data.temperature * 9.0 / 5.0) + 32.0, "°F"),
                _ => (data.temperature, "°C"), // Default to celsius
            };

            // Check if data is stale (older than 2x update interval)
            let stale_threshold = self.update_interval * 2;
            let is_stale = self.last_update.elapsed() > stale_threshold;

            let stale_indicator = if is_stale { " (stale)" } else { "" };

            // Show error indicator if there's an error but we have old data
            let error_indicator = if self.error_message.is_some() {
                " ⚠"
            } else {
                ""
            };

            format!(
                "{}{} {} | {}% humidity{}{}",
                temp.round(),
                unit,
                data.condition,
                data.humidity,
                stale_indicator,
                error_indicator
            )
        })
    }

    pub async fn fetch_weather(&mut self) -> WeatherResult<()> {
        // Validate API key is configured
        if self.api_key.is_empty() {
            warn!("Weather API key not configured");
            return Err(WeatherError::NoApiKey);
        }

        info!(city = %self.city, "Fetching weather from API");

        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            self.city, self.api_key
        );

        let response = reqwest::get(&url).await.map_err(|e| {
            warn!(error = %e, city = %self.city, "Failed to fetch weather from API");
            e
        })?;

        // Check if the response indicates city not found
        if !response.status().is_success() {
            warn!(city = %self.city, status = %response.status(), "Weather API returned error status");
            return Err(WeatherError::CityNotFound(self.city.clone()));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            warn!(error = %e, "Failed to parse weather API response");
            e
        })?;

        // Parse the response with better error handling
        let temperature = json["main"]["temp"]
            .as_f64()
            .ok_or_else(|| WeatherError::ParseError("temperature".to_string()))?
            as f32;

        let condition = json["weather"][0]["main"]
            .as_str()
            .ok_or_else(|| WeatherError::ParseError("condition".to_string()))?
            .to_string();

        let humidity = json["main"]["humidity"]
            .as_u64()
            .ok_or_else(|| WeatherError::ParseError("humidity".to_string()))?
            as u32;

        let wind_speed = json["wind"]["speed"]
            .as_f64()
            .ok_or_else(|| WeatherError::ParseError("wind_speed".to_string()))?
            as f32;

        self.data = Some(WeatherData {
            temperature,
            condition: condition.clone(),
            humidity,
            wind_speed,
        });

        self.last_update = std::time::Instant::now();

        info!(
            temperature = %temperature,
            condition = %condition,
            "Weather API fetch successful"
        );

        Ok(())
    }
}

// Implement the Widget trait for WeatherWidget
impl Widget for WeatherWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "weather",
            name: "Weather",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Delegate to existing update method
        WeatherWidget::update(self);
    }

    fn content(&self) -> WidgetContent {
        match (&self.data, self.display_string()) {
            (Some(data), Some(text)) => {
                let icon = Self::condition_to_icon(&data.condition);
                WidgetContent::IconText {
                    icon: icon.to_string(),
                    text,
                    size: FontSize::Medium,
                }
            }
            (None, Some(text)) => {
                // Error case - no icon, just text
                WidgetContent::Text {
                    text,
                    size: FontSize::Medium,
                }
            }
            _ => WidgetContent::Empty,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        self.data.is_some() || self.error_message.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_widget() {
        let clock = ClockWidget::new("24h", true, false);
        assert!(!clock.time_string().is_empty());
    }

    #[test]
    fn test_clock_widget_new() {
        let clock = ClockWidget::new("24h", true, false);
        assert!(!clock.time_string().is_empty());
    }

    #[test]
    fn test_clock_widget_12h_format() {
        let clock = ClockWidget::new("12h", true, false);
        let time = clock.time_string();
        // 12h format should contain AM or PM
        assert!(time.contains("AM") || time.contains("PM"));
    }

    #[test]
    fn test_clock_widget_24h_format() {
        let clock = ClockWidget::new("24h", true, false);
        let time = clock.time_string();
        // 24h format should not contain AM or PM
        assert!(!time.contains("AM") && !time.contains("PM"));
    }

    #[test]
    fn test_clock_widget_no_seconds() {
        let clock = ClockWidget::new("24h", false, false);
        let time = clock.time_string();
        // Without seconds, format is HH:MM (5 chars)
        // With seconds, format is HH:MM:SS (8 chars)
        assert_eq!(time.len(), 5);
    }

    #[test]
    fn test_clock_widget_update() {
        let mut clock = ClockWidget::new("24h", true, false);
        let _old_time = clock.time_string();
        // Force update
        clock.last_update = std::time::Instant::now() - std::time::Duration::from_secs(2);
        clock.update();
        // Time string should still be valid (may or may not have changed)
        assert!(!clock.time_string().is_empty());
    }

    #[test]
    fn test_clock_date_string() {
        let clock = ClockWidget::new("24h", true, true);
        let date = clock.date_string();
        // Date should contain year
        assert!(date.contains("202"));
    }

    #[test]
    fn test_weather_widget() {
        let weather = WeatherWidget::new("London", "test_key", "celsius", 600);
        assert_eq!(weather.city, "London");
    }

    #[test]
    fn test_weather_widget_set_data() {
        let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);
        let data = WeatherData {
            temperature: 20.5,
            condition: "Cloudy".to_string(),
            humidity: 70,
            wind_speed: 10.0,
        };
        weather.set_data(data);
        assert!(weather.data.is_some());
        assert!(weather.error_message.is_none());
    }

    #[test]
    fn test_weather_widget_set_error() {
        let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);
        weather.set_error("API Error".to_string());
        assert!(weather.error_message.is_some());
    }

    #[test]
    fn test_weather_widget_display_string_with_error_no_data() {
        let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);
        weather.set_error("Connection failed".to_string());
        let display = weather.display_string();
        assert!(display.is_some());
        assert!(display.unwrap().contains("Error"));
    }

    #[test]
    fn test_weather_condition_to_icon() {
        assert_eq!(WeatherWidget::condition_to_icon("Clear"), "weather-clear");
        assert_eq!(WeatherWidget::condition_to_icon("Sunny"), "weather-clear");
        assert_eq!(WeatherWidget::condition_to_icon("Clouds"), "weather-clouds");
        assert_eq!(WeatherWidget::condition_to_icon("Rain"), "weather-rain");
        assert_eq!(WeatherWidget::condition_to_icon("Snow"), "weather-snow");
        assert_eq!(
            WeatherWidget::condition_to_icon("Thunderstorm"),
            "weather-storm"
        );
        assert_eq!(
            WeatherWidget::condition_to_icon("Unknown"),
            "weather-clouds"
        );
    }

    #[test]
    fn test_weather_widget_content_with_icon() {
        let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);
        let data = WeatherData {
            temperature: 20.5,
            condition: "Clear".to_string(),
            humidity: 70,
            wind_speed: 10.0,
        };
        weather.set_data(data);

        match weather.content() {
            WidgetContent::IconText { icon, .. } => {
                assert_eq!(icon, "weather-clear");
            }
            _ => panic!("Expected IconText variant"),
        }
    }
}
