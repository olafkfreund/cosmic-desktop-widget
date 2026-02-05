//! Stub implementation when audio feature is disabled

use std::path::PathBuf;
use thiserror::Error;
use tracing::debug;

/// Errors that can occur during audio playback
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Audio feature not enabled")]
    NotEnabled,
}

/// Sound effect specification
#[derive(Debug, Clone)]
pub enum SoundEffect {
    /// Built-in sound by name
    Builtin(String),
    /// Custom sound from file path
    Custom(PathBuf),
}

impl Default for SoundEffect {
    fn default() -> Self {
        SoundEffect::Builtin("notification".to_string())
    }
}

impl SoundEffect {
    /// Parse a sound effect from configuration string
    pub fn from_config(value: &str) -> Self {
        if value.contains('/') || value.ends_with(".ogg") || value.ends_with(".wav") {
            SoundEffect::Custom(PathBuf::from(value))
        } else {
            SoundEffect::Builtin(value.to_string())
        }
    }
}

/// Stub audio player (does nothing when audio feature is disabled)
pub struct AudioPlayer;

impl AudioPlayer {
    /// Create a new audio player stub
    pub fn new() -> Result<Self, AudioError> {
        debug!("Audio feature not enabled, using stub player");
        Ok(Self)
    }

    /// Set volume (no-op)
    pub fn set_volume(&mut self, _volume: f32) {}

    /// Play sound (no-op)
    pub fn play(&self, _sound: &SoundEffect) -> Result<(), AudioError> {
        debug!("Audio playback skipped (feature not enabled)");
        Ok(())
    }

    /// Play repeated sound (no-op)
    pub fn play_repeated(&self, _sound: &SoundEffect, _count: u32) -> Result<(), AudioError> {
        Ok(())
    }

    /// Preview sound (no-op)
    pub fn preview(&self, _sound: &SoundEffect) -> Result<(), AudioError> {
        Ok(())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self
    }
}
