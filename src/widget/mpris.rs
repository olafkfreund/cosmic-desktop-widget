//! MPRIS (Media Player Remote Interfacing Specification) widget
//!
//! Displays currently playing media from D-Bus MPRIS interface.
//! Shows artist, title, album, and playback status from active media players.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};
use zbus::{fdo::DBusProxy, Connection};

use super::registry::DynWidgetFactory;
use super::traits::{FontSize, Widget, WidgetContent, WidgetInfo};

/// MPRIS metadata for currently playing track
#[derive(Debug, Clone, Default)]
struct MprisMetadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    playback_status: PlaybackStatus,
}

/// Playback status from MPRIS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

impl PlaybackStatus {
    fn icon(&self) -> &'static str {
        match self {
            PlaybackStatus::Playing => ">",
            PlaybackStatus::Paused => "||",
            PlaybackStatus::Stopped => "[]",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// Configuration for MPRIS widget
#[derive(Debug, Clone)]
pub struct MprisConfig {
    pub show_artist: bool,
    pub show_album: bool,
    pub show_status: bool,
    pub preferred_player: Option<String>,
    pub max_length: usize,
    pub update_interval: u64,
}

impl Default for MprisConfig {
    fn default() -> Self {
        Self {
            show_artist: true,
            show_album: false,
            show_status: true,
            preferred_player: None,
            max_length: 50,
            update_interval: 1,
        }
    }
}

/// MPRIS widget showing currently playing media
pub struct MprisWidget {
    config: MprisConfig,
    metadata: Arc<Mutex<MprisMetadata>>,
    last_update: Instant,
    update_interval: Duration,
    error_message: Option<String>,
}

impl MprisWidget {
    /// Create a new MPRIS widget with default configuration
    pub fn new() -> Self {
        Self::with_config(MprisConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: MprisConfig) -> Self {
        let update_interval = Duration::from_secs(config.update_interval);
        let metadata = Arc::new(Mutex::new(MprisMetadata::default()));

        // Spawn background task to fetch MPRIS data (only if tokio runtime is available)
        let metadata_clone = Arc::clone(&metadata);
        let preferred_player = config.preferred_player.clone();

        // Check if we're running in a tokio context
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                if let Err(e) = Self::mpris_update_loop(metadata_clone, preferred_player).await {
                    warn!(error = %e, "MPRIS update loop failed");
                }
            });
        } else {
            debug!("No tokio runtime available, MPRIS updates will be disabled");
        }

        Self {
            config,
            metadata,
            last_update: Instant::now(),
            update_interval,
            error_message: None,
        }
    }

    /// Background task to continuously update MPRIS data
    async fn mpris_update_loop(
        metadata: Arc<Mutex<MprisMetadata>>,
        preferred_player: Option<String>,
    ) -> Result<()> {
        loop {
            match Self::fetch_mpris_data(preferred_player.as_deref()).await {
                Ok(new_metadata) => {
                    if let Ok(mut guard) = metadata.lock() {
                        *guard = new_metadata;
                    }
                }
                Err(e) => {
                    debug!(error = %e, "Failed to fetch MPRIS data");
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Fetch MPRIS data from D-Bus
    async fn fetch_mpris_data(preferred_player: Option<&str>) -> Result<MprisMetadata> {
        let connection = Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;

        // Find active media players
        let player_name = Self::find_active_player(&connection, preferred_player).await?;

        // Get player proxy
        let player_proxy = zbus::Proxy::new(
            &connection,
            player_name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await
        .context("Failed to create player proxy")?;

        // Get playback status
        let status_str = player_proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .unwrap_or_else(|_| "Stopped".to_string());
        let playback_status = PlaybackStatus::from_str(&status_str);

        // Get metadata
        use zbus::zvariant::OwnedValue;
        let metadata_variant = player_proxy
            .get_property::<OwnedValue>("Metadata")
            .await
            .context("Failed to get Metadata")?;

        let mut artist = None;
        let mut title = None;
        let mut album = None;

        // Parse metadata dictionary using TryFrom
        if let Ok(dict) = <std::collections::HashMap<String, OwnedValue>>::try_from(metadata_variant) {
            // Get artist (array of strings) - try to extract as Vec<String>
            if let Some(artist_val) = dict.get("xesam:artist") {
                if let Ok(owned) = artist_val.try_clone() {
                    // Try array of strings first, then single string
                    if let Ok(owned2) = owned.try_clone() {
                        if let Ok(artists) = Vec::<String>::try_from(owned2) {
                            artist = artists.into_iter().next();
                        } else if let Ok(s) = String::try_from(owned) {
                            artist = Some(s);
                        }
                    }
                }
            }

            // Get title (string)
            if let Some(title_val) = dict.get("xesam:title") {
                if let Ok(owned) = title_val.try_clone() {
                    if let Ok(s) = String::try_from(owned) {
                        title = Some(s);
                    }
                }
            }

            // Get album (string)
            if let Some(album_val) = dict.get("xesam:album") {
                if let Ok(owned) = album_val.try_clone() {
                    if let Ok(s) = String::try_from(owned) {
                        album = Some(s);
                    }
                }
            }
        }

        Ok(MprisMetadata {
            artist,
            title,
            album,
            playback_status,
        })
    }

    /// Find an active media player on the bus
    async fn find_active_player(
        connection: &Connection,
        preferred: Option<&str>,
    ) -> Result<String> {
        let dbus_proxy = DBusProxy::new(connection)
            .await
            .context("Failed to create DBus proxy")?;

        let names = dbus_proxy
            .list_names()
            .await
            .context("Failed to list D-Bus names")?;

        let mut mpris_players: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
            .map(|s| s.to_string())
            .collect();

        if mpris_players.is_empty() {
            anyhow::bail!("No MPRIS players found");
        }

        // Check for preferred player
        if let Some(pref) = preferred {
            let pref_name = format!("org.mpris.MediaPlayer2.{}", pref);
            if mpris_players.iter().any(|p| p == &pref_name) {
                debug!(player = %pref_name, "Using preferred player");
                return Ok(pref_name);
            }
        }

        // Return first available player
        mpris_players.sort();
        let player = mpris_players[0].clone();
        debug!(player = %player, "Using first available player");
        Ok(player)
    }

    /// Format display string based on configuration
    fn format_display(&self, metadata: &MprisMetadata) -> String {
        let mut parts = Vec::new();

        // Add status icon if configured
        if self.config.show_status && metadata.playback_status != PlaybackStatus::Stopped {
            parts.push(format!("{}", metadata.playback_status.icon()));
        }

        // Build main content
        let mut content_parts = Vec::new();

        if self.config.show_artist {
            if let Some(artist) = &metadata.artist {
                content_parts.push(artist.clone());
            }
        }

        if let Some(title) = &metadata.title {
            content_parts.push(title.clone());
        }

        if self.config.show_album {
            if let Some(album) = &metadata.album {
                content_parts.push(format!("({})", album));
            }
        }

        if content_parts.is_empty() {
            return "No media playing".to_string();
        }

        let content =
            if self.config.show_artist && metadata.artist.is_some() && metadata.title.is_some() {
                // Format as "Artist - Title"
                content_parts.join(" - ")
            } else {
                content_parts.join(" ")
            };

        // Add music note emoji at start
        if !parts.is_empty() {
            parts.push("🎵".to_string());
        }

        parts.push(content);

        let display = parts.join(" ");

        // Truncate if too long
        if display.len() > self.config.max_length {
            format!(
                "{}...",
                &display[..self.config.max_length.saturating_sub(3)]
            )
        } else {
            display
        }
    }

    /// Get current display string
    pub fn display_string(&self) -> String {
        if let Some(error) = &self.error_message {
            return format!("Error: {}", error);
        }

        if let Ok(metadata) = self.metadata.lock() {
            if metadata.title.is_none() && metadata.playback_status == PlaybackStatus::Stopped {
                return "No media playing".to_string();
            }
            self.format_display(&metadata)
        } else {
            "No media playing".to_string()
        }
    }
}

impl Default for MprisWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for MprisWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "mpris",
            name: "Now Playing (MPRIS)",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        // Background task handles updates
        // This is just called periodically by the framework
        self.last_update = Instant::now();
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        self.update_interval
    }

    fn is_ready(&self) -> bool {
        true // Always ready, shows "No media playing" if nothing available
    }

    fn error(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Factory for MprisWidget
pub struct MprisWidgetFactory;

impl DynWidgetFactory for MprisWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "mpris"
    }

    fn create(&self, config: &toml::Table) -> Result<Box<dyn Widget>> {
        let show_artist = config
            .get("show_artist")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let show_album = config
            .get("show_album")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let show_status = config
            .get("show_status")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let preferred_player = config
            .get("preferred_player")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let max_length = config
            .get("max_length")
            .and_then(|v| v.as_integer())
            .unwrap_or(50) as usize;

        let update_interval = config
            .get("update_interval")
            .and_then(|v| v.as_integer())
            .unwrap_or(1) as u64;

        info!(
            show_artist = %show_artist,
            show_album = %show_album,
            show_status = %show_status,
            preferred_player = ?preferred_player,
            max_length = %max_length,
            update_interval = %update_interval,
            "Creating MprisWidget"
        );

        let widget_config = MprisConfig {
            show_artist,
            show_album,
            show_status,
            preferred_player,
            max_length,
            update_interval,
        };

        Ok(Box::new(MprisWidget::with_config(widget_config)))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("show_artist".to_string(), toml::Value::Boolean(true));
        config.insert("show_album".to_string(), toml::Value::Boolean(false));
        config.insert("show_status".to_string(), toml::Value::Boolean(true));
        config.insert("max_length".to_string(), toml::Value::Integer(50));
        config.insert("update_interval".to_string(), toml::Value::Integer(1));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> Result<()> {
        if let Some(max_len) = config.get("max_length") {
            let val = max_len
                .as_integer()
                .context("'max_length' must be an integer")?;

            if val < 10 {
                anyhow::bail!("'max_length' must be at least 10 characters");
            }
        }

        if let Some(interval) = config.get("update_interval") {
            let val = interval
                .as_integer()
                .context("'update_interval' must be an integer")?;

            if val < 1 {
                anyhow::bail!("'update_interval' must be at least 1 second");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_status_icon() {
        assert_eq!(PlaybackStatus::Playing.icon(), ">");
        assert_eq!(PlaybackStatus::Paused.icon(), "||");
        assert_eq!(PlaybackStatus::Stopped.icon(), "[]");
    }

    #[test]
    fn test_playback_status_from_str() {
        assert_eq!(PlaybackStatus::from_str("Playing"), PlaybackStatus::Playing);
        assert_eq!(PlaybackStatus::from_str("Paused"), PlaybackStatus::Paused);
        assert_eq!(PlaybackStatus::from_str("Stopped"), PlaybackStatus::Stopped);
        assert_eq!(PlaybackStatus::from_str("Unknown"), PlaybackStatus::Stopped);
    }

    #[test]
    fn test_mpris_widget_creation() {
        let widget = MprisWidget::new();
        assert_eq!(widget.info().id, "mpris");
        assert_eq!(widget.info().name, "Now Playing (MPRIS)");
    }

    #[test]
    fn test_mpris_config_default() {
        let config = MprisConfig::default();
        assert!(config.show_artist);
        assert!(!config.show_album);
        assert!(config.show_status);
        assert_eq!(config.max_length, 50);
        assert_eq!(config.update_interval, 1);
    }

    #[test]
    fn test_format_display_no_media() {
        let widget = MprisWidget::new();
        let metadata = MprisMetadata::default();
        let display = widget.format_display(&metadata);
        assert_eq!(display, "No media playing");
    }

    #[test]
    fn test_format_display_with_title_only() {
        let widget = MprisWidget::new();
        let metadata = MprisMetadata {
            artist: None,
            title: Some("Test Song".to_string()),
            album: None,
            playback_status: PlaybackStatus::Playing,
        };
        let display = widget.format_display(&metadata);
        assert!(display.contains("Test Song"));
        assert!(display.contains("🎵"));
    }

    #[test]
    fn test_format_display_with_artist_and_title() {
        let widget = MprisWidget::new();
        let metadata = MprisMetadata {
            artist: Some("Test Artist".to_string()),
            title: Some("Test Song".to_string()),
            album: None,
            playback_status: PlaybackStatus::Playing,
        };
        let display = widget.format_display(&metadata);
        assert!(display.contains("Test Artist"));
        assert!(display.contains("Test Song"));
        assert!(display.contains(" - "));
    }

    #[test]
    fn test_format_display_truncate() {
        let config = MprisConfig {
            max_length: 20,
            ..Default::default()
        };
        let widget = MprisWidget::with_config(config);
        let metadata = MprisMetadata {
            artist: Some("Very Long Artist Name".to_string()),
            title: Some("Very Long Song Title That Should Be Truncated".to_string()),
            album: None,
            playback_status: PlaybackStatus::Playing,
        };
        let display = widget.format_display(&metadata);
        assert!(display.len() <= 20);
        assert!(display.ends_with("..."));
    }

    #[test]
    fn test_factory_creation() {
        let factory = MprisWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "mpris");
    }

    #[test]
    fn test_factory_validation() {
        let factory = MprisWidgetFactory;

        // Valid config
        let mut valid = toml::Table::new();
        valid.insert("max_length".to_string(), toml::Value::Integer(50));
        assert!(factory.validate_config(&valid).is_ok());

        // Invalid max_length
        let mut invalid = toml::Table::new();
        invalid.insert("max_length".to_string(), toml::Value::Integer(5));
        assert!(factory.validate_config(&invalid).is_err());

        // Invalid update_interval
        let mut invalid_interval = toml::Table::new();
        invalid_interval.insert("update_interval".to_string(), toml::Value::Integer(0));
        assert!(factory.validate_config(&invalid_interval).is_err());
    }
}
