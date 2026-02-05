//! Configuration migration from old format to new format
//!
//! This module handles automatic migration of config files from the old
//! flat structure to the new hierarchical format with widget instances.

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info};

use super::{Config, Margin, PanelConfig};
use crate::position::Position;
use crate::widget::WidgetInstance;

/// Old configuration format (pre-widget-registry)
#[derive(Debug, Clone, Deserialize)]
pub struct OldConfig {
    /// Widget width in pixels
    #[serde(default = "default_width")]
    pub width: u32,

    /// Widget height in pixels
    #[serde(default = "default_height")]
    pub height: u32,

    /// Position: "top-left", "top-right", "bottom-left", "bottom-right", "center"
    #[serde(default = "default_position")]
    pub position: String,

    /// Margins from screen edges
    #[serde(default)]
    pub margin: OldMargin,

    /// Weather city
    #[serde(default = "default_city")]
    pub weather_city: String,

    /// OpenWeatherMap API key
    #[serde(default)]
    pub weather_api_key: String,

    /// Update interval in seconds
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,

    /// Show clock
    #[serde(default = "default_true")]
    pub show_clock: bool,

    /// Show weather
    #[serde(default = "default_true")]
    pub show_weather: bool,

    /// Clock format: "12h" or "24h"
    #[serde(default = "default_clock_format")]
    pub clock_format: String,

    /// Temperature unit: "celsius" or "fahrenheit"
    #[serde(default = "default_temp_unit")]
    pub temperature_unit: String,

    /// Show seconds in clock
    #[serde(default = "default_true")]
    pub show_seconds: bool,

    /// Show date in clock
    #[serde(default = "default_false")]
    pub show_date: bool,

    /// Theme name
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Layout padding in pixels
    #[serde(default = "default_padding")]
    pub padding: f32,

    /// Spacing between widgets
    #[serde(default = "default_spacing")]
    pub spacing: f32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OldMargin {
    #[serde(default = "default_margin_top")]
    pub top: i32,
    #[serde(default = "default_margin_right")]
    pub right: i32,
    #[serde(default)]
    pub bottom: i32,
    #[serde(default)]
    pub left: i32,
}

// Default value functions
fn default_width() -> u32 {
    400
}
fn default_height() -> u32 {
    150
}
fn default_position() -> String {
    "top-right".to_string()
}
fn default_city() -> String {
    "London".to_string()
}
fn default_update_interval() -> u64 {
    600
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_clock_format() -> String {
    "24h".to_string()
}
fn default_temp_unit() -> String {
    "celsius".to_string()
}
fn default_theme() -> String {
    "cosmic_dark".to_string()
}
fn default_padding() -> f32 {
    20.0
}
fn default_spacing() -> f32 {
    10.0
}
fn default_margin_top() -> i32 {
    20
}
fn default_margin_right() -> i32 {
    20
}

/// Migrate from old config format to new format
///
/// Returns None if the content doesn't appear to be old format.
pub fn migrate_from_old_format(content: &str) -> Result<Config> {
    // Try to parse as old format
    let old: OldConfig = toml::from_str(content).context("Failed to parse old config format")?;

    info!(
        width = old.width,
        height = old.height,
        show_clock = old.show_clock,
        show_weather = old.show_weather,
        "Migrating from old config format"
    );

    // Convert to new format
    let config = convert_old_to_new(old);

    debug!("Migration complete");
    Ok(config)
}

/// Check if content looks like old config format
pub fn is_old_format(content: &str) -> bool {
    // Old format has flat structure with show_clock, show_weather
    // New format has [panel] section and [[widgets]] array
    content.contains("show_clock")
        || content.contains("show_weather")
        || (content.contains("width") && !content.contains("[panel]"))
}

/// Convert old config to new format
fn convert_old_to_new(old: OldConfig) -> Config {
    let mut widgets = Vec::new();

    // Create clock widget if enabled
    if old.show_clock {
        let mut clock_config = toml::Table::new();
        clock_config.insert(
            "format".to_string(),
            toml::Value::String(old.clock_format.clone()),
        );
        clock_config.insert(
            "show_seconds".to_string(),
            toml::Value::Boolean(old.show_seconds),
        );
        clock_config.insert("show_date".to_string(), toml::Value::Boolean(old.show_date));

        widgets.push(WidgetInstance::with_config("clock", clock_config));
    }

    // Create weather widget if enabled
    if old.show_weather {
        let mut weather_config = toml::Table::new();
        weather_config.insert(
            "city".to_string(),
            toml::Value::String(old.weather_city.clone()),
        );
        weather_config.insert(
            "api_key".to_string(),
            toml::Value::String(old.weather_api_key.clone()),
        );
        weather_config.insert(
            "temperature_unit".to_string(),
            toml::Value::String(old.temperature_unit.clone()),
        );
        weather_config.insert(
            "update_interval".to_string(),
            toml::Value::Integer(old.update_interval as i64),
        );

        widgets.push(WidgetInstance::with_config("weather", weather_config));
    }

    Config {
        panel: PanelConfig {
            width: old.width,
            height: old.height,
            position: old.position.parse::<Position>().unwrap_or_default(),
            margin: Margin {
                top: old.margin.top,
                right: old.margin.right,
                bottom: old.margin.bottom,
                left: old.margin.left,
            },
            theme: old.theme,
            background_opacity: None,
            padding: old.padding,
            spacing: old.spacing,
        },
        widgets,
        custom_theme: None,
        theme_config: None,
        sounds: super::SoundsConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_old_format() {
        let old_config = r#"
            width = 400
            height = 150
            show_clock = true
            show_weather = true
        "#;
        assert!(is_old_format(old_config));

        let new_config = r#"
            [panel]
            width = 450
            height = 180

            [[widgets]]
            type = "clock"
        "#;
        assert!(!is_old_format(new_config));
    }

    #[test]
    fn test_migrate_old_format() {
        let old_config = r#"
            width = 400
            height = 150
            position = "top-right"
            show_clock = true
            show_weather = true
            weather_city = "Paris"
            clock_format = "12h"

            [margin]
            top = 20
            right = 20
        "#;

        let config = migrate_from_old_format(old_config).unwrap();

        assert_eq!(config.panel.width, 400);
        assert_eq!(config.panel.height, 150);
        assert_eq!(config.widgets.len(), 2);

        // Check clock widget
        let clock = &config.widgets[0];
        assert_eq!(clock.widget_type, "clock");
        assert_eq!(clock.config.get("format").unwrap().as_str().unwrap(), "12h");

        // Check weather widget
        let weather = &config.widgets[1];
        assert_eq!(weather.widget_type, "weather");
        assert_eq!(
            weather.config.get("city").unwrap().as_str().unwrap(),
            "Paris"
        );
    }

    #[test]
    fn test_migrate_clock_only() {
        let old_config = r#"
            width = 300
            height = 100
            show_clock = true
            show_weather = false
        "#;

        let config = migrate_from_old_format(old_config).unwrap();
        assert_eq!(config.widgets.len(), 1);
        assert_eq!(config.widgets[0].widget_type, "clock");
    }

    #[test]
    fn test_migrate_weather_only() {
        let old_config = r#"
            width = 300
            height = 100
            show_clock = false
            show_weather = true
            weather_city = "Tokyo"
        "#;

        let config = migrate_from_old_format(old_config).unwrap();
        assert_eq!(config.widgets.len(), 1);
        assert_eq!(config.widgets[0].widget_type, "weather");
    }

    #[test]
    fn test_migrate_preserves_settings() {
        let old_config = r#"
            width = 500
            height = 200
            position = "bottom-left"
            theme = "light"
            padding = 30.0
            spacing = 15.0
            show_clock = true
            show_weather = true
            clock_format = "24h"
            show_seconds = false
            show_date = true
            weather_city = "Berlin"
            temperature_unit = "fahrenheit"
            update_interval = 300

            [margin]
            top = 10
            right = 10
            bottom = 10
            left = 10
        "#;

        let config = migrate_from_old_format(old_config).unwrap();

        // Panel settings
        assert_eq!(config.panel.width, 500);
        assert_eq!(config.panel.height, 200);
        assert_eq!(config.panel.position.as_str(), "bottom-left");
        assert_eq!(config.panel.theme, "light");
        assert!((config.panel.padding - 30.0).abs() < 0.01);
        assert!((config.panel.spacing - 15.0).abs() < 0.01);

        // Margin
        assert_eq!(config.panel.margin.top, 10);
        assert_eq!(config.panel.margin.right, 10);

        // Clock config
        let clock = &config.widgets[0];
        assert_eq!(clock.config.get("format").unwrap().as_str().unwrap(), "24h");
        assert!(!clock.config.get("show_seconds").unwrap().as_bool().unwrap());
        assert!(clock.config.get("show_date").unwrap().as_bool().unwrap());

        // Weather config
        let weather = &config.widgets[1];
        assert_eq!(
            weather.config.get("city").unwrap().as_str().unwrap(),
            "Berlin"
        );
        assert_eq!(
            weather
                .config
                .get("temperature_unit")
                .unwrap()
                .as_str()
                .unwrap(),
            "fahrenheit"
        );
        assert_eq!(
            weather
                .config
                .get("update_interval")
                .unwrap()
                .as_integer()
                .unwrap(),
            300
        );
    }
}
