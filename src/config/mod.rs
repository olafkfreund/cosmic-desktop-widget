// Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            
            toml::from_str(&content)
                .context("Failed to parse config file")
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            Ok(config)
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
