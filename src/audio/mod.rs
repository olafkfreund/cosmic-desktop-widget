//! Audio playback system for alarms and notifications
//!
//! This module provides audio playback capabilities for the widget system,
//! supporting both built-in sounds and custom audio files.

#[cfg(feature = "audio")]
mod player;

#[cfg(feature = "audio")]
pub use player::{AudioPlayer, SoundEffect};

#[cfg(not(feature = "audio"))]
mod stub;

#[cfg(not(feature = "audio"))]
pub use stub::{AudioPlayer, SoundEffect};

use serde::{Deserialize, Serialize};

/// Sound configuration for widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundConfig {
    /// Whether sound is enabled
    #[serde(default = "default_sound_enabled")]
    pub enabled: bool,

    /// Sound effect to play
    #[serde(default)]
    pub effect: String,

    /// Volume level (0.0 to 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,

    /// Number of times to repeat the sound
    #[serde(default = "default_repeat")]
    pub repeat: u32,
}

fn default_sound_enabled() -> bool {
    false
}

fn default_volume() -> f32 {
    0.8
}

fn default_repeat() -> u32 {
    1
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            effect: "notification".to_string(),
            volume: 0.8,
            repeat: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_config_default() {
        let config = SoundConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.volume, 0.8);
        assert_eq!(config.repeat, 1);
    }
}
