// Configuration management

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Widget width in pixels
    pub width: u32,
    
    /// Widget height in pixels
    pub height: u32,
    
    /// Position: "top-left", "top-right", "bottom-left", "bottom-right", "center"
    pub position: String,
    
    /// Margins from screen edges
    pub margin: Margin,
    
    /// Weather city
    pub weather_city: String,
    
    /// OpenWeatherMap API key
    pub weather_api_key: String,
    
    /// Update interval in seconds
    pub update_interval: u64,
    
    /// Show clock
    pub show_clock: bool,
    
    /// Show weather
    pub show_weather: bool,
    
    /// Clock format: "12h" or "24h"
    pub clock_format: String,

    /// Temperature unit: "celsius" or "fahrenheit"
    pub temperature_unit: String,

    /// Show seconds in clock
    pub show_seconds: bool,

    /// Show date in clock
    pub show_date: bool,

    /// Theme name: "cosmic_dark", "light", "transparent_dark", or "custom"
    pub theme: String,

    /// Custom theme settings (used when theme = "custom")
    pub custom_theme: Option<Theme>,

    /// Layout padding in pixels
    pub padding: f32,

    /// Spacing between widgets
    pub spacing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margin {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 400,
            height: 150,
            position: "top-right".to_string(),
            margin: Margin {
                top: 20,
                right: 20,
                bottom: 0,
                left: 0,
            },
            weather_city: "London".to_string(),
            weather_api_key: String::new(),
            update_interval: 600, // 10 minutes
            show_clock: true,
            show_weather: true,
            clock_format: "24h".to_string(),
            temperature_unit: "celsius".to_string(),
            show_seconds: true,
            show_date: false,
            theme: "cosmic_dark".to_string(),
            custom_theme: None,
            padding: 20.0,
            spacing: 10.0,
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    ///
    /// This method is resilient to errors - if config file is missing, corrupted,
    /// or invalid, it will use defaults and log a warning.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str::<Config>(&content) {
                        Ok(config) => {
                            // Validate the loaded config
                            if let Err(e) = config.validate() {
                                tracing::warn!(
                                    error = %e,
                                    "Config validation failed, using defaults"
                                );
                                return Ok(Self::default());
                            }
                            Ok(config)
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "Failed to parse config file, using defaults"
                            );
                            Ok(Self::default())
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to read config file, using defaults"
                    );
                    Ok(Self::default())
                }
            }
        } else {
            // Create default config
            let config = Self::default();
            if let Err(e) = config.save() {
                tracing::warn!(
                    error = %e,
                    "Failed to save default config, continuing anyway"
                );
            }
            Ok(config)
        }
    }

    /// Validate configuration values
    ///
    /// Returns an error if any configuration value is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            bail!("Width and height must be positive (got {}x{})", self.width, self.height);
        }

        if self.width > 10000 || self.height > 10000 {
            bail!("Width and height are unreasonably large (got {}x{})", self.width, self.height);
        }

        if self.update_interval == 0 {
            bail!("Update interval must be positive (got {})", self.update_interval);
        }

        if self.update_interval < 60 {
            tracing::warn!(
                interval = self.update_interval,
                "Update interval is very short, this may exceed API rate limits"
            );
        }

        let valid_positions = ["top-left", "top-right", "bottom-left", "bottom-right", "center"];
        if !valid_positions.contains(&self.position.as_str()) {
            bail!(
                "Invalid position '{}', must be one of: {}",
                self.position,
                valid_positions.join(", ")
            );
        }

        let valid_formats = ["12h", "24h"];
        if !valid_formats.contains(&self.clock_format.as_str()) {
            bail!(
                "Invalid clock format '{}', must be one of: {}",
                self.clock_format,
                valid_formats.join(", ")
            );
        }

        let valid_units = ["celsius", "fahrenheit"];
        if !valid_units.contains(&self.temperature_unit.as_str()) {
            bail!(
                "Invalid temperature unit '{}', must be one of: {}",
                self.temperature_unit,
                valid_units.join(", ")
            );
        }

        Ok(())
    }

    /// Get the theme based on configuration
    pub fn get_theme(&self) -> Theme {
        if self.theme == "custom" {
            self.custom_theme.clone().unwrap_or_default()
        } else {
            Theme::from_name(&self.theme)
        }
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(&config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }
    
    /// Get the path to the configuration file
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?;
        
        Ok(config_dir
            .join("cosmic-desktop-widget")
            .join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.width, 400);
        assert_eq!(config.height, 150);
        assert!(config.show_clock);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config.width, deserialized.width);
    }
}
