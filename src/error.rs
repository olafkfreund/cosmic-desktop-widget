// Error types for COSMIC Desktop Widget
//
// This module defines error types using thiserror for better error handling
// and debugging throughout the application.

use thiserror::Error;

/// Main error type for widget operations
#[derive(Error, Debug)]
pub enum WidgetError {
    #[error("Wayland connection failed: {0}")]
    WaylandConnection(String),

    #[error("Layer shell not available")]
    LayerShellNotAvailable,

    #[error("Buffer creation failed: {0}")]
    BufferCreation(String),

    #[error("Rendering failed: {0}")]
    RenderError(String),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("Weather API error: {0}")]
    WeatherError(#[from] WeatherError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Config directory not found")]
    NoConfigDir,

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
}

/// Weather widget errors
#[derive(Error, Debug)]
pub enum WeatherError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    #[error("API key not configured")]
    NoApiKey,

    #[error("Failed to parse weather data: {0}")]
    ParseError(String),

    #[error("City not found: {0}")]
    CityNotFound(String),
}

// Convenience type aliases for common Result types
pub type Result<T> = std::result::Result<T, WidgetError>;
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub type WeatherResult<T> = std::result::Result<T, WeatherError>;
