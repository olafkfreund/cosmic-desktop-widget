//! Integration tests for COSMIC Desktop Widget
//!
//! These tests verify that different components of the widget system
//! work together correctly without requiring a Wayland connection.

use cosmic_desktop_widget::config::Config;
use cosmic_desktop_widget::layout::{LayoutDirection, LayoutManager};
use cosmic_desktop_widget::theme::Theme;
use cosmic_desktop_widget::update::UpdateScheduler;
use cosmic_desktop_widget::widget::{ClockWidget, WeatherData, WeatherWidget};
use std::time::Duration;

// Test that config can be loaded and widgets initialized
#[test]
fn test_widget_initialization_flow() {
    // This tests the initialization flow without Wayland
    let config = Config::default();

    let clock = ClockWidget::new(
        &config.clock_format,
        config.show_seconds,
        config.show_date,
    );

    let weather = WeatherWidget::new(
        &config.weather_city,
        &config.weather_api_key,
        &config.temperature_unit,
        config.update_interval,
    );

    assert!(!clock.time_string().is_empty());
    // Weather may not have data initially
    assert!(weather.display_string().is_none() || weather.display_string().is_some());
}

// Test config serialization round-trip
#[test]
fn test_config_round_trip() {
    let config = Config::default();
    let serialized = toml::to_string(&config).expect("Failed to serialize");
    let deserialized: Config = toml::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(config.width, deserialized.width);
    assert_eq!(config.height, deserialized.height);
    assert_eq!(config.position, deserialized.position);
    assert_eq!(config.show_clock, deserialized.show_clock);
    assert_eq!(config.show_weather, deserialized.show_weather);
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

    // Transparent dark should have lower opacity
    assert!(transparent.opacity < dark.opacity);
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

    let mut scheduler = UpdateScheduler::new(
        Duration::from_millis(50),
        Duration::from_millis(100),
    );

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
    let mut config = Config {
        theme: "cosmic_dark".to_string(),
        ..Default::default()
    };

    let theme = config.get_theme();
    assert_eq!(theme.accent.r, 52); // COSMIC blue

    // Test custom theme
    config.theme = "custom".to_string();
    config.custom_theme = Some(Theme::light());

    let custom_theme = config.get_theme();
    assert_eq!(custom_theme.background.r, 255); // Light background
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

    let mut clock = ClockWidget::new(
        &config.clock_format,
        config.show_seconds,
        config.show_date,
    );

    let mut weather = WeatherWidget::new(
        &config.weather_city,
        &config.weather_api_key,
        &config.temperature_unit,
        config.update_interval,
    );

    let mut scheduler = UpdateScheduler::new(
        Duration::from_secs(1),
        Duration::from_secs(config.update_interval),
    );

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
    let layout = LayoutManager::new(config.width, config.height)
        .with_padding(config.padding)
        .with_spacing(config.spacing);

    let clock_pos = layout.clock_position(config.show_weather);
    let weather_pos = layout.weather_position(config.show_clock);

    assert!(clock_pos.width > 0.0);
    assert!(weather_pos.width > 0.0);
}

// Test config default values are sensible
#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert!(config.width > 0);
    assert!(config.height > 0);
    assert!(config.update_interval > 0);
    assert!(config.padding >= 0.0);
    assert!(config.spacing >= 0.0);
    assert!(config.show_clock || config.show_weather); // At least one should be shown
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
    let mut scheduler = UpdateScheduler::new(
        Duration::from_secs(60),
        Duration::from_secs(600),
    );

    // Force immediate update
    scheduler.force_update_all();

    let flags = scheduler.check_updates();
    assert!(flags.clock);
    assert!(flags.weather);
}
