//! Audio player implementation using rodio

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur during audio playback
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to create output stream: {0}")]
    StreamError(String),

    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Sound file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unknown builtin sound: {0}")]
    UnknownBuiltin(String),

    #[error("Audio system not available")]
    NotAvailable,
}

/// Sound effect specification
#[derive(Debug, Clone)]
pub enum SoundEffect {
    /// Built-in sound by name (alarm, chime, notification)
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
        // Check if it's a path
        if value.contains('/') || value.contains('\\') || value.ends_with(".ogg") || value.ends_with(".wav") || value.ends_with(".mp3") {
            SoundEffect::Custom(PathBuf::from(value))
        } else {
            SoundEffect::Builtin(value.to_string())
        }
    }
}

/// Audio player for playing sounds
pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    volume: f32,
}

impl AudioPlayer {
    /// Create a new audio player
    pub fn new() -> Result<Self, AudioError> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        info!("Audio player initialized");

        Ok(Self {
            _stream: stream,
            stream_handle,
            volume: 0.8,
        })
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Play a sound effect
    pub fn play(&self, sound: &SoundEffect) -> Result<(), AudioError> {
        match sound {
            SoundEffect::Builtin(name) => self.play_builtin(name),
            SoundEffect::Custom(path) => self.play_file(path),
        }
    }

    /// Play a sound effect multiple times
    pub fn play_repeated(&self, sound: &SoundEffect, count: u32) -> Result<(), AudioError> {
        for i in 0..count {
            self.play(sound)?;
            if i < count - 1 {
                // Small delay between repetitions
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        Ok(())
    }

    /// Play a built-in sound by name
    fn play_builtin(&self, name: &str) -> Result<(), AudioError> {
        debug!(sound = %name, "Playing builtin sound");

        // Generate simple tones for built-in sounds
        // In a real implementation, you'd embed actual sound files
        let (frequency, duration_ms) = match name {
            "alarm" => (880.0, 500),       // A5, 500ms
            "chime" => (523.25, 200),      // C5, 200ms
            "notification" => (659.25, 150), // E5, 150ms
            "beep" => (440.0, 100),        // A4, 100ms
            _ => {
                warn!(sound = %name, "Unknown builtin sound, using default beep");
                (440.0, 100)
            }
        };

        // Generate a simple sine wave tone
        let sample_rate = 44100u32;
        let duration = Duration::from_millis(duration_ms);
        let samples: Vec<f32> = (0..((sample_rate as u64 * duration.as_millis() as u64) / 1000) as usize)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // Simple sine wave with envelope
                let envelope = if t < 0.01 {
                    t / 0.01
                } else if t > duration.as_secs_f32() - 0.05 {
                    (duration.as_secs_f32() - t) / 0.05
                } else {
                    1.0
                };
                (t * frequency * 2.0 * std::f32::consts::PI).sin() * envelope * self.volume
            })
            .collect();

        let source = SamplesSource::new(samples, sample_rate);

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        sink.append(source);
        sink.detach();

        Ok(())
    }

    /// Play a sound from a file
    fn play_file(&self, path: &PathBuf) -> Result<(), AudioError> {
        debug!(path = %path.display(), "Playing sound file");

        if !path.exists() {
            return Err(AudioError::FileNotFound(path.clone()));
        }

        let file = std::fs::File::open(path)
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let source = Decoder::new(std::io::BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        sink.set_volume(self.volume);
        sink.append(source);
        sink.detach();

        Ok(())
    }

    /// Preview a sound effect (play once at current volume)
    pub fn preview(&self, sound: &SoundEffect) -> Result<(), AudioError> {
        self.play(sound)
    }
}

/// Simple samples-based audio source for generated tones
struct SamplesSource {
    samples: Vec<f32>,
    position: usize,
    sample_rate: u32,
}

impl SamplesSource {
    fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            position: 0,
            sample_rate,
        }
    }
}

impl Iterator for SamplesSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for SamplesSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> u16 {
        1 // Mono
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let samples_remaining = self.samples.len() - self.position;
        Some(Duration::from_secs_f32(
            samples_remaining as f32 / self.sample_rate as f32,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_effect_from_config() {
        let builtin = SoundEffect::from_config("alarm");
        assert!(matches!(builtin, SoundEffect::Builtin(s) if s == "alarm"));

        let custom = SoundEffect::from_config("/path/to/sound.ogg");
        assert!(matches!(custom, SoundEffect::Custom(_)));

        let custom_wav = SoundEffect::from_config("sound.wav");
        assert!(matches!(custom_wav, SoundEffect::Custom(_)));
    }

    #[test]
    fn test_sound_effect_default() {
        let effect = SoundEffect::default();
        assert!(matches!(effect, SoundEffect::Builtin(s) if s == "notification"));
    }
}
