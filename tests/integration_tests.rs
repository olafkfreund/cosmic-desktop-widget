//! Integration tests for COSMIC Desktop Widget
//!
//! These tests verify that different components of the widget system
//! work together correctly without requiring a Wayland connection.

use cosmic_desktop_widget::config::Config;
use cosmic_desktop_widget::layout::{LayoutDirection, LayoutManager};
use cosmic_desktop_widget::theme::Theme;
use cosmic_desktop_widget::update::UpdateScheduler;
use cosmic_desktop_widget::widget::{ClockWidget, WeatherData, WeatherWidget, WidgetRegistry};
use std::time::Duration;

// Test that config can be loaded and widgets initialized using registry
#[test]
fn test_widget_initialization_flow() {
    // This tests the initialization flow without Wayland
    let config = Config::default();
    let registry = WidgetRegistry::with_builtins();

    // Create widgets from config
    for instance in config.enabled_widgets() {
        let widget = registry.create(&instance.widget_type, &instance.config);
        assert!(
            widget.is_ok(),
            "Failed to create widget: {}",
            instance.widget_type
        );
    }
}

// Test config serialization round-trip
#[test]
fn test_config_round_trip() {
    let config = Config::default();
    let serialized = toml::to_string(&config).expect("Failed to serialize");
    let deserialized: Config = toml::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(config.panel.width, deserialized.panel.width);
    assert_eq!(config.panel.height, deserialized.panel.height);
    assert_eq!(config.panel.position, deserialized.panel.position);
    assert_eq!(config.widgets.len(), deserialized.widgets.len());
}

// Test theme loading
#[test]
fn test_theme_loading() {
    let dark = Theme::from_name("cosmic_dark");
    let light = Theme::from_name("light");
    let transparent = Theme::from_name("transparent_dark");
    let unknown = Theme::from_name("nonexistent");

    // Unknown should default to cosmic_dark
    assert_eq!(dark.accent.r, unknown.accent.r);

    // Light theme should have lighter background
    assert!(light.background.r > dark.background.r);

    // Transparent dark should have lower background alpha (opacity baked in)
    assert!(transparent.background.a < dark.background.a);
}

// Test layout calculations
#[test]
fn test_layout_calculations() {
    let layout = LayoutManager::new(400, 200)
        .with_padding(20.0)
        .with_spacing(10.0)
        .with_direction(LayoutDirection::Vertical);

    let positions = layout.calculate_positions(&[40.0, 30.0]);

    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].x, 20.0);
    assert_eq!(positions[0].y, 20.0);
    assert_eq!(positions[1].y, 70.0); // 20 + 40 + 10

    // Test horizontal layout
    let layout_h = LayoutManager::new(400, 200)
        .with_padding(20.0)
        .with_spacing(10.0)
        .with_direction(LayoutDirection::Horizontal);

    let positions_h = layout_h.calculate_positions(&[40.0, 30.0]);
    assert_eq!(positions_h.len(), 2);
    assert!(positions_h[1].x > positions_h[0].x);
}

// Test update scheduler timing
#[test]
fn test_update_scheduler_timing() {
    use std::thread;

    let mut scheduler = UpdateScheduler::new(Duration::from_millis(50), Duration::from_millis(100));

    // Initially should not need update
    let flags = scheduler.check_updates();
    assert!(!flags.clock);
    assert!(!flags.weather);

    // Wait for clock interval
    thread::sleep(Duration::from_millis(60));
    let flags = scheduler.check_updates();
    assert!(flags.clock, "Clock should need update after interval");

    // Weather should still not need update
    assert!(!flags.weather, "Weather should not need update yet");
}

// Test clock widget update behavior
#[test]
fn test_clock_widget_integration() {
    let mut clock = ClockWidget::new("24h", true, false);
    let _initial_time = clock.time_string();

    // Force update by manipulating internal state through update method
    std::thread::sleep(Duration::from_millis(1100));
    clock.update();

    // Time should still be valid
    assert!(!clock.time_string().is_empty());

    // Test 12h format
    let clock_12h = ClockWidget::new("12h", true, false);
    let time_12h = clock_12h.time_string();
    assert!(time_12h.contains("AM") || time_12h.contains("PM"));
}

// Test weather widget data handling
#[test]
fn test_weather_widget_data_handling() {
    let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);

    // Initially no data
    assert!(weather.display_string().is_none());

    // Set weather data
    let data = WeatherData {
        temperature: 20.5,
        condition: "Cloudy".to_string(),
        humidity: 70,
        wind_speed: 10.0,
    };
    weather.set_data(data);

    // Should now have display string
    let display = weather.display_string();
    assert!(display.is_some());

    let text = display.unwrap();
    assert!(text.contains("21")); // Rounded temperature
    assert!(text.contains("°C"));
    assert!(text.contains("Cloudy"));
    assert!(text.contains("70%"));
}

// Test weather widget error handling
#[test]
fn test_weather_widget_error_handling() {
    let mut weather = WeatherWidget::new("London", "test_key", "celsius", 600);

    // Set error
    weather.set_error("API Error".to_string());

    // Should show error
    let display = weather.display_string();
    assert!(display.is_some());
    assert!(display.unwrap().contains("Error"));

    // Now set data - should clear error
    let data = WeatherData {
        temperature: 15.0,
        condition: "Sunny".to_string(),
        humidity: 50,
        wind_speed: 5.0,
    };
    weather.set_data(data);

    let display = weather.display_string();
    assert!(display.is_some());
    assert!(!display.unwrap().contains("Error"));
}

// Test theme integration with config
#[test]
fn test_theme_config_integration() {
    let mut config = Config::default();
    config.panel.theme = "cosmic_dark".to_string();

    let theme = config.get_theme();
    assert_eq!(theme.accent.r, 52); // COSMIC blue

    // Test custom theme
    config.panel.theme = "custom".to_string();
    config.custom_theme = Some(Theme::light());

    let custom_theme = config.get_theme();
    assert_eq!(custom_theme.background.r, 248); // Light background
}

// Test layout with widget positioning
#[test]
fn test_layout_widget_positioning() {
    let layout = LayoutManager::new(400, 150);

    // Clock with weather
    let clock_pos = layout.clock_position(true);
    assert_eq!(clock_pos.y, 20.0);

    let weather_pos = layout.weather_position(true);
    assert!(weather_pos.y > clock_pos.y);

    // Clock without weather (centered)
    let clock_pos_centered = layout.clock_position(false);
    assert!(clock_pos_centered.y > clock_pos.y);
}

// Test update flags
#[test]
fn test_update_flags_integration() {
    use cosmic_desktop_widget::update::UpdateFlags;

    let mut flags = UpdateFlags::new();
    assert!(!flags.needs_redraw());

    flags.clock = true;
    assert!(flags.needs_redraw());

    flags.set_all();
    assert!(flags.clock);
    assert!(flags.weather);
    assert!(flags.layout);
    assert!(flags.theme);

    flags.clear();
    assert!(!flags.needs_redraw());
}

// Test complete widget rendering workflow
#[test]
fn test_widget_rendering_workflow() {
    // This simulates the workflow in main.rs without Wayland
    let config = Config::default();
    let _theme = config.get_theme();

    let mut clock = ClockWidget::new("24h", true, false);
    let mut weather = WeatherWidget::new("London", "", "celsius", 600);

    let mut scheduler = UpdateScheduler::new(Duration::from_secs(1), Duration::from_secs(600));

    // Simulate first update
    let flags = scheduler.check_updates();
    if flags.clock {
        clock.update();
    }
    if flags.weather {
        weather.update();
    }

    // Check clock has valid output
    assert!(!clock.time_string().is_empty());

    // Layout should work
    let layout = LayoutManager::new(config.panel.width, config.panel.height)
        .with_padding(config.panel.padding)
        .with_spacing(config.panel.spacing);

    let clock_pos = layout.clock_position(true);
    let weather_pos = layout.weather_position(true);

    assert!(clock_pos.width > 0.0);
    assert!(weather_pos.width > 0.0);
}

// Test config default values are sensible
#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert!(config.panel.width > 0);
    assert!(config.panel.height > 0);
    assert!(config.panel.padding >= 0.0);
    assert!(config.panel.spacing >= 0.0);
    assert!(!config.widgets.is_empty()); // Should have some widgets
}

// Test temperature unit conversion
#[test]
fn test_temperature_unit_conversion() {
    let mut weather = WeatherWidget::new("London", "test_key", "fahrenheit", 600);

    let data = WeatherData {
        temperature: 0.0, // 0°C
        condition: "Cold".to_string(),
        humidity: 80,
        wind_speed: 15.0,
    };
    weather.set_data(data);

    let display = weather.display_string().unwrap();
    assert!(display.contains("32°F")); // 0°C = 32°F

    // Test celsius
    let mut weather_c = WeatherWidget::new("London", "test_key", "celsius", 600);
    let data_c = WeatherData {
        temperature: 25.0,
        condition: "Warm".to_string(),
        humidity: 60,
        wind_speed: 5.0,
    };
    weather_c.set_data(data_c);

    let display_c = weather_c.display_string().unwrap();
    assert!(display_c.contains("25°C"));
}

// Test force update functionality
#[test]
fn test_force_update() {
    let mut scheduler = UpdateScheduler::new(Duration::from_secs(60), Duration::from_secs(600));

    // Force immediate update
    scheduler.force_update_all();

    let flags = scheduler.check_updates();
    assert!(flags.clock);
    assert!(flags.weather);
}

// Test widget registry
#[test]
fn test_widget_registry() {
    let registry = WidgetRegistry::with_builtins();

    // Check built-in widgets are registered
    assert!(registry.has_widget("clock"));
    assert!(registry.has_widget("weather"));
    assert!(registry.has_widget("system_monitor"));
    assert!(registry.has_widget("countdown"));
    assert!(registry.has_widget("quotes"));

    // Unknown widget should not exist
    assert!(!registry.has_widget("nonexistent"));
}

// Test creating widgets from registry
#[test]
fn test_registry_widget_creation() {
    let registry = WidgetRegistry::with_builtins();

    // Create clock with default config
    let clock = registry.create_default("clock");
    assert!(clock.is_ok());

    // Create weather with custom config
    let mut weather_config = toml::Table::new();
    weather_config.insert("city".to_string(), toml::Value::String("Paris".to_string()));
    let weather = registry.create("weather", &weather_config);
    assert!(weather.is_ok());

    // Create system monitor
    let sysmon = registry.create_default("system_monitor");
    assert!(sysmon.is_ok());
}

// Test config migration
#[test]
fn test_config_migration() {
    use cosmic_desktop_widget::config::migration;

    let old_config = r#"
        width = 400
        height = 150
        position = "top-right"
        show_clock = true
        show_weather = true
        weather_city = "Berlin"
        clock_format = "12h"
        show_seconds = false
        temperature_unit = "fahrenheit"
        update_interval = 300

        [margin]
        top = 20
        right = 20
    "#;

    let config = migration::migrate_from_old_format(old_config).unwrap();

    // Panel settings should be preserved
    assert_eq!(config.panel.width, 400);
    assert_eq!(config.panel.height, 150);
    assert_eq!(config.panel.position.as_str(), "top-right");

    // Should have 2 widgets (clock and weather)
    assert_eq!(config.widgets.len(), 2);

    // Check clock config
    let clock = &config.widgets[0];
    assert_eq!(clock.widget_type, "clock");
    assert_eq!(clock.config.get("format").unwrap().as_str().unwrap(), "12h");
    assert!(!clock.config.get("show_seconds").unwrap().as_bool().unwrap());

    // Check weather config
    let weather = &config.widgets[1];
    assert_eq!(weather.widget_type, "weather");
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
}

// Test new widget types
#[test]
fn test_new_widgets() {
    use cosmic_desktop_widget::widget::{
        CountdownWidget, QuotesWidget, SystemMonitorWidget, Widget,
    };

    // System Monitor
    let sysmon = SystemMonitorWidget::default();
    assert_eq!(sysmon.info().id, "system_monitor");
    let display = sysmon.display_string();
    assert!(display.contains("CPU:") || display.contains("RAM:"));

    // Countdown
    let target = chrono::Local::now() + chrono::Duration::days(10);
    let countdown = CountdownWidget::new("Event", target, true, true, true, false);
    assert_eq!(countdown.info().id, "countdown");
    let display = countdown.display_string();
    assert!(display.contains("Event:"));

    // Quotes
    let quotes = QuotesWidget::default();
    assert_eq!(quotes.info().id, "quotes");
    let display = quotes.display_string();
    assert!(!display.is_empty());
}
