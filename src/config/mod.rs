// Configuration management

use crate::audio::SoundConfig;
use crate::position::Position;
use crate::theme::Theme;
use crate::widget::WidgetInstance;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod migration;

/// Panel configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// Widget width in pixels
    pub width: u32,

    /// Widget height in pixels
    pub height: u32,

    /// Widget position on screen
    ///
    /// Supported positions:
    /// - Top row: "top-left", "top-center", "top-right"
    /// - Middle row: "center-left", "center", "center-right"
    /// - Bottom row: "bottom-left", "bottom-center", "bottom-right"
    pub position: Position,

    /// Margins from screen edges
    pub margin: Margin,

    /// Theme name: "cosmic_dark", "light", "transparent_dark", "transparent_light", "glass", or "custom"
    pub theme: String,

    /// Override background opacity (0.0 = fully transparent, 1.0 = fully opaque)
    /// If not set, uses the theme's default opacity
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_opacity: Option<f32>,

    /// Layout padding in pixels
    pub padding: f32,

    /// Spacing between widgets
    pub spacing: f32,
}

/// Extended theme configuration for custom themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Base colors
    #[serde(default)]
    pub colors: ThemeColors,

    /// Style options
    #[serde(default)]
    pub style: ThemeStyle,

    /// Gradient configuration (optional)
    #[serde(default)]
    pub gradient: Option<GradientConfig>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            colors: ThemeColors::default(),
            style: ThemeStyle::default(),
            gradient: None,
        }
    }
}

/// Theme color settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Background color (hex string like "#1e1e2e")
    #[serde(default = "default_background_color")]
    pub background: String,

    /// Primary text color
    #[serde(default = "default_text_primary_color")]
    pub text_primary: String,

    /// Secondary text color
    #[serde(default = "default_text_secondary_color")]
    pub text_secondary: String,

    /// Accent color
    #[serde(default = "default_accent_color")]
    pub accent: String,

    /// Border color
    #[serde(default = "default_border_color")]
    pub border: String,
}

fn default_background_color() -> String {
    "#1e1e2e".to_string()
}

fn default_text_primary_color() -> String {
    "#cdd6f4".to_string()
}

fn default_text_secondary_color() -> String {
    "#a6adc8".to_string()
}

fn default_accent_color() -> String {
    "#89b4fa".to_string()
}

fn default_border_color() -> String {
    "#45475a".to_string()
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: default_background_color(),
            text_primary: default_text_primary_color(),
            text_secondary: default_text_secondary_color(),
            accent: default_accent_color(),
            border: default_border_color(),
        }
    }
}

/// Theme style settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStyle {
    /// Background opacity (0.0 to 1.0)
    #[serde(default = "default_opacity")]
    pub background_opacity: f32,

    /// Corner radius in pixels
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,

    /// Border width in pixels
    #[serde(default = "default_border_width")]
    pub border_width: f32,

    /// Enable compositor blur hint
    #[serde(default)]
    pub blur_enabled: bool,
}

fn default_opacity() -> f32 {
    0.85
}

fn default_corner_radius() -> f32 {
    12.0
}

fn default_border_width() -> f32 {
    1.0
}

impl Default for ThemeStyle {
    fn default() -> Self {
        Self {
            background_opacity: default_opacity(),
            corner_radius: default_corner_radius(),
            border_width: default_border_width(),
            blur_enabled: false,
        }
    }
}

/// Gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientConfig {
    /// Whether gradient is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Start color (hex string)
    #[serde(default = "default_background_color")]
    pub start_color: String,

    /// End color (hex string)
    #[serde(default = "default_gradient_end_color")]
    pub end_color: String,

    /// Gradient angle in degrees (0-360)
    #[serde(default = "default_gradient_angle")]
    pub angle: f32,
}

fn default_gradient_end_color() -> String {
    "#313244".to_string()
}

fn default_gradient_angle() -> f32 {
    135.0
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_color: default_background_color(),
            end_color: default_gradient_end_color(),
            angle: default_gradient_angle(),
        }
    }
}

/// Global sound settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundsConfig {
    /// Whether sounds are enabled globally
    #[serde(default)]
    pub enabled: bool,

    /// Master volume (0.0 to 1.0)
    #[serde(default = "default_master_volume")]
    pub volume: f32,

    /// Alarm sound settings
    #[serde(default)]
    pub alarm: SoundConfig,

    /// Notification sound settings
    #[serde(default)]
    pub notification: SoundConfig,
}

fn default_master_volume() -> f32 {
    0.8
}

impl Default for SoundsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: default_master_volume(),
            alarm: SoundConfig {
                enabled: true,
                effect: "alarm".to_string(),
                volume: 0.8,
                repeat: 3,
            },
            notification: SoundConfig {
                enabled: true,
                effect: "notification".to_string(),
                volume: 0.7,
                repeat: 1,
            },
        }
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            width: 450,
            height: 180,
            position: Position::default(), // TopRight
            margin: Margin::default(),
            theme: "cosmic_dark".to_string(),
            background_opacity: None,
            padding: 20.0,
            spacing: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Margin {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 10,
            right: 20,
            bottom: 0,
            left: 0,
        }
    }
}

/// Main configuration structure (new format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Panel settings
    pub panel: PanelConfig,

    /// Ordered list of widget instances
    #[serde(default = "default_widgets")]
    pub widgets: Vec<WidgetInstance>,

    /// Custom theme settings (used when theme = "custom")
    pub custom_theme: Option<Theme>,

    /// Extended theme configuration (for GUI theme editor)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_config: Option<ThemeConfig>,

    /// Sound settings
    #[serde(default)]
    pub sounds: SoundsConfig,
}

fn default_widgets() -> Vec<WidgetInstance> {
    vec![
        WidgetInstance::with_config("clock", {
            let mut config = toml::Table::new();
            config.insert("format".to_string(), toml::Value::String("24h".to_string()));
            config.insert("show_seconds".to_string(), toml::Value::Boolean(true));
            config.insert("show_date".to_string(), toml::Value::Boolean(false));
            config
        }),
        WidgetInstance::with_config("weather", {
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
        }),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panel: PanelConfig::default(),
            widgets: default_widgets(),
            custom_theme: None,
            theme_config: None,
            sounds: SoundsConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    ///
    /// This method handles migration from old config format automatically.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    // Try new format first
                    if let Ok(config) = toml::from_str::<Config>(&content) {
                        if let Err(e) = config.validate() {
                            tracing::warn!(
                                error = %e,
                                "Config validation failed, using defaults"
                            );
                            return Ok(Self::default());
                        }
                        return Ok(config);
                    }

                    // Try migrating from old format
                    match migration::migrate_from_old_format(&content) {
                        Ok(config) => {
                            tracing::info!("Migrated config from old format");
                            // Save migrated config
                            if let Err(e) = config.save() {
                                tracing::warn!(error = %e, "Failed to save migrated config");
                            }
                            return Ok(config);
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "Failed to parse or migrate config file, using defaults"
                            );
                            return Ok(Self::default());
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to read config file, using defaults"
                    );
                    return Ok(Self::default());
                }
            }
        }

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

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate panel settings
        if self.panel.width == 0 || self.panel.height == 0 {
            bail!(
                "Width and height must be positive (got {}x{})",
                self.panel.width,
                self.panel.height
            );
        }

        if self.panel.width > 10000 || self.panel.height > 10000 {
            bail!(
                "Width and height are unreasonably large (got {}x{})",
                self.panel.width,
                self.panel.height
            );
        }

        // Position is now type-safe, no validation needed

        // Validate widgets
        if self.widgets.is_empty() {
            tracing::warn!("No widgets configured");
        }

        Ok(())
    }

    /// Get the theme based on configuration
    pub fn get_theme(&self) -> Theme {
        let mut theme = if self.panel.theme == "custom" {
            self.custom_theme.clone().unwrap_or_default()
        } else {
            Theme::from_name(&self.panel.theme)
        };

        // Apply opacity override if set
        if let Some(opacity) = self.panel.background_opacity {
            theme.opacity = opacity.clamp(0.0, 1.0);
        }

        theme
    }

    /// Get enabled widgets in order
    pub fn enabled_widgets(&self) -> impl Iterator<Item = &WidgetInstance> {
        self.widgets.iter().filter(|w| w.enabled)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get the path to the configuration file
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;

        Ok(config_dir.join("cosmic-desktop-widget").join("config.toml"))
    }

    // Legacy accessors for backward compatibility with existing code
    // These will be removed once main.rs is updated

    /// Get widget width (legacy accessor)
    pub fn width(&self) -> u32 {
        self.panel.width
    }

    /// Get widget height (legacy accessor)
    pub fn height(&self) -> u32 {
        self.panel.height
    }

    /// Get position (legacy accessor)
    #[deprecated(since = "0.2.0", note = "Use panel.position directly")]
    pub fn position(&self) -> &str {
        self.panel.position.as_str()
    }

    /// Get position as Position enum
    pub fn position_enum(&self) -> Position {
        self.panel.position
    }

    /// Get margin (legacy accessor)
    pub fn margin(&self) -> &Margin {
        &self.panel.margin
    }

    /// Get padding (legacy accessor)
    pub fn padding(&self) -> f32 {
        self.panel.padding
    }

    /// Get spacing (legacy accessor)
    pub fn spacing(&self) -> f32 {
        self.panel.spacing
    }

    /// Get theme name (legacy accessor)
    pub fn theme(&self) -> &str {
        &self.panel.theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.panel.width, 450);
        assert_eq!(config.panel.height, 180);
        assert!(!config.widgets.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config.panel.width, deserialized.panel.width);
    }

    #[test]
    fn test_enabled_widgets() {
        let mut config = Config::default();
        config.widgets[0].enabled = false;

        let enabled: Vec<_> = config.enabled_widgets().collect();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn test_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());

        let mut invalid = Config::default();
        invalid.panel.width = 0;
        assert!(invalid.validate().is_err());
    }
}
