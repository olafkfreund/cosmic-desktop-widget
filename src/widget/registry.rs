//! Widget registry for dynamic widget creation
//!
//! This module provides the infrastructure for registering and creating widgets
//! dynamically based on configuration. It supports:
//!
//! - Type-erased widget factories
//! - Registration of built-in and custom widgets
//! - Creation of widgets from TOML configuration

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tracing::{debug, info, warn};

use super::battery::BatteryWidgetFactory;
use super::calendar::CalendarWidgetFactory;
use super::countdown::CountdownWidgetFactory;
use super::crypto::CryptoWidgetFactory;
use super::mpris::MprisWidgetFactory;
use super::news::NewsWidgetFactory;
use super::pomodoro::PomodoroWidgetFactory;
use super::quotes::QuotesWidgetFactory;
use super::stocks::StocksWidgetFactory;
use super::system_monitor::SystemMonitorWidgetFactory;
use super::traits::Widget;
use super::{ClockWidget, WeatherWidget};

/// Type-erased widget factory trait
///
/// This trait allows storing different widget factories in a single collection
/// without knowing the concrete types at compile time.
pub trait DynWidgetFactory: Send + Sync {
    /// The widget type identifier (e.g., "clock", "weather")
    fn widget_type(&self) -> &'static str;

    /// Create a new widget instance from TOML configuration
    fn create(&self, config: &toml::Table) -> Result<Box<dyn Widget>>;

    /// Get default configuration for this widget type
    fn default_config(&self) -> toml::Table;

    /// Validate configuration before creating widget
    fn validate_config(&self, config: &toml::Table) -> Result<()>;
}

/// Registry for widget factories
///
/// The registry holds all available widget factories and creates widget
/// instances based on configuration.
pub struct WidgetRegistry {
    factories: HashMap<&'static str, Arc<dyn DynWidgetFactory>>,
}

impl WidgetRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Create a registry with all built-in widgets registered
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();

        // Register built-in widgets
        registry.register(BatteryWidgetFactory);
        registry.register(CalendarWidgetFactory);
        registry.register(ClockWidgetFactory);
        registry.register(WeatherWidgetFactory);
        registry.register(SystemMonitorWidgetFactory);
        registry.register(CountdownWidgetFactory);
        registry.register(CryptoWidgetFactory);
        registry.register(MprisWidgetFactory);
        registry.register(NewsWidgetFactory);
        registry.register(PomodoroWidgetFactory);
        registry.register(QuotesWidgetFactory);
        registry.register(StocksWidgetFactory);

        info!(
            widget_types = ?registry.factories.keys().collect::<Vec<_>>(),
            "Widget registry initialized with built-in widgets"
        );

        registry
    }

    /// Register a widget factory
    pub fn register<F: DynWidgetFactory + 'static>(&mut self, factory: F) {
        let widget_type = factory.widget_type();
        debug!(widget_type = %widget_type, "Registering widget factory");
        self.factories.insert(widget_type, Arc::new(factory));
    }

    /// Check if a widget type is registered
    pub fn has_widget(&self, widget_type: &str) -> bool {
        self.factories.contains_key(widget_type)
    }

    /// Get all registered widget types
    pub fn widget_types(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }

    /// Create a widget from configuration
    pub fn create(&self, widget_type: &str, config: &toml::Table) -> Result<Box<dyn Widget>> {
        let factory = self.factories.get(widget_type).with_context(|| {
            format!(
                "Unknown widget type: '{}'. Available types: {:?}",
                widget_type,
                self.widget_types()
            )
        })?;

        // Validate configuration first
        factory
            .validate_config(config)
            .with_context(|| format!("Invalid configuration for widget type '{}'", widget_type))?;

        factory
            .create(config)
            .with_context(|| format!("Failed to create widget of type '{}'", widget_type))
    }

    /// Create a widget with default configuration
    pub fn create_default(&self, widget_type: &str) -> Result<Box<dyn Widget>> {
        let factory = self
            .factories
            .get(widget_type)
            .with_context(|| format!("Unknown widget type: '{}'", widget_type))?;

        let config = factory.default_config();
        factory.create(&config)
    }

    /// Get default configuration for a widget type
    pub fn default_config(&self, widget_type: &str) -> Result<toml::Table> {
        let factory = self
            .factories
            .get(widget_type)
            .with_context(|| format!("Unknown widget type: '{}'", widget_type))?;

        Ok(factory.default_config())
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ============================================================================
// Built-in Widget Factories
// ============================================================================

/// Factory for ClockWidget
pub struct ClockWidgetFactory;

impl DynWidgetFactory for ClockWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "clock"
    }

    fn create(&self, config: &toml::Table) -> Result<Box<dyn Widget>> {
        let format = config
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("24h");

        let show_seconds = config
            .get("show_seconds")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_date = config
            .get("show_date")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!(
            format = %format,
            show_seconds = %show_seconds,
            show_date = %show_date,
            "Creating ClockWidget"
        );

        Ok(Box::new(ClockWidget::new(format, show_seconds, show_date)))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("format".to_string(), toml::Value::String("24h".to_string()));
        config.insert("show_seconds".to_string(), toml::Value::Boolean(true));
        config.insert("show_date".to_string(), toml::Value::Boolean(false));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> Result<()> {
        if let Some(format) = config.get("format") {
            let format_str = format.as_str().context("'format' must be a string")?;

            if format_str != "12h" && format_str != "24h" {
                bail!("'format' must be '12h' or '24h', got '{}'", format_str);
            }
        }
        Ok(())
    }
}

/// Factory for WeatherWidget
pub struct WeatherWidgetFactory;

impl DynWidgetFactory for WeatherWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "weather"
    }

    fn create(&self, config: &toml::Table) -> Result<Box<dyn Widget>> {
        let city = config
            .get("city")
            .and_then(|v| v.as_str())
            .unwrap_or("London");

        let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

        let temperature_unit = config
            .get("temperature_unit")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius");

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(600) as u64;

        debug!(
            city = %city,
            temperature_unit = %temperature_unit,
            update_interval = %update_interval,
            has_api_key = !api_key.is_empty(),
            "Creating WeatherWidget"
        );

        Ok(Box::new(WeatherWidget::new(
            city,
            api_key,
            temperature_unit,
            update_interval,
        )))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert(
            "city".to_string(),
            toml::Value::String("London".to_string()),
        );
        config.insert("api_key".to_string(), toml::Value::String(String::new()));
        config.insert(
            "temperature_unit".to_string(),
            toml::Value::String("celsius".to_string()),
        );
        config.insert("update_interval".to_string(), toml::Value::Integer(600));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> Result<()> {
        if let Some(unit) = config.get("temperature_unit") {
            let unit_str = unit
                .as_str()
                .context("'temperature_unit' must be a string")?;

            if unit_str != "celsius" && unit_str != "fahrenheit" {
                bail!(
                    "'temperature_unit' must be 'celsius' or 'fahrenheit', got '{}'",
                    unit_str
                );
            }
        }

        if let Some(interval) = config.get("update_interval") {
            let interval_val = interval
                .as_integer()
                .context("'update_interval' must be an integer")?;

            if interval_val < 60 {
                warn!("Weather update interval ({} seconds) is very short, may exceed API rate limits", interval_val);
            }
        }

        Ok(())
    }
}

// ============================================================================
// Widget Instance Configuration
// ============================================================================

/// Configuration for a single widget instance
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetInstance {
    /// Widget type identifier (e.g., "clock", "weather")
    #[serde(rename = "type")]
    pub widget_type: String,

    /// Whether this widget is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Optional unique identifier for this instance
    #[serde(default)]
    pub id: Option<String>,

    /// Widget-specific configuration
    #[serde(default)]
    pub config: toml::Table,
}

fn default_true() -> bool {
    true
}

impl WidgetInstance {
    /// Create a new widget instance configuration
    pub fn new(widget_type: &str) -> Self {
        Self {
            widget_type: widget_type.to_string(),
            enabled: true,
            id: None,
            config: toml::Table::new(),
        }
    }

    /// Create with specific configuration
    pub fn with_config(widget_type: &str, config: toml::Table) -> Self {
        Self {
            widget_type: widget_type.to_string(),
            enabled: true,
            id: None,
            config,
        }
    }

    /// Get a unique identifier for this instance
    pub fn instance_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| self.widget_type.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_builtins() {
        let registry = WidgetRegistry::with_builtins();
        assert!(registry.has_widget("clock"));
        assert!(registry.has_widget("weather"));
        assert!(!registry.has_widget("nonexistent"));
    }

    #[test]
    fn test_create_clock_widget() {
        let registry = WidgetRegistry::with_builtins();
        let widget = registry.create_default("clock").unwrap();
        assert_eq!(widget.info().id, "clock");
    }

    #[test]
    fn test_create_weather_widget() {
        let registry = WidgetRegistry::with_builtins();
        let widget = registry.create_default("weather").unwrap();
        assert_eq!(widget.info().id, "weather");
    }

    #[test]
    fn test_clock_with_custom_config() {
        let registry = WidgetRegistry::with_builtins();
        let mut config = toml::Table::new();
        config.insert("format".to_string(), toml::Value::String("12h".to_string()));
        config.insert("show_seconds".to_string(), toml::Value::Boolean(false));

        let widget = registry.create("clock", &config).unwrap();
        assert_eq!(widget.info().id, "clock");
    }

    #[test]
    fn test_invalid_widget_type() {
        let registry = WidgetRegistry::with_builtins();
        let result = registry.create("invalid_type", &toml::Table::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_widget_types_list() {
        let registry = WidgetRegistry::with_builtins();
        let types = registry.widget_types();
        assert!(types.contains(&"clock"));
        assert!(types.contains(&"weather"));
    }

    #[test]
    fn test_widget_instance() {
        let instance = WidgetInstance::new("clock");
        assert_eq!(instance.widget_type, "clock");
        assert!(instance.enabled);
        assert!(instance.id.is_none());
    }

    #[test]
    fn test_clock_config_validation() {
        let factory = ClockWidgetFactory;

        // Valid config
        let mut valid = toml::Table::new();
        valid.insert("format".to_string(), toml::Value::String("12h".to_string()));
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid format
        let mut invalid = toml::Table::new();
        invalid.insert(
            "format".to_string(),
            toml::Value::String("invalid".to_string()),
        );
        assert!(factory.validate_config(&invalid).is_err());
    }
}
