// Configuration file watcher with hot-reload capability

use anyhow::{Context, Result};
use notify::{
    event::{EventKind, ModifyKind},
    Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Configuration reload event
#[derive(Debug, Clone)]
pub struct ConfigReloadEvent {
    /// Timestamp when the event was generated
    pub timestamp: Instant,
}

/// Configuration file watcher
///
/// Monitors the config file for changes and sends reload events through a channel.
/// Implements debouncing to avoid multiple reloads for rapid file changes (common
/// with text editors that save multiple times).
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<ConfigReloadEvent>,
}

impl ConfigWatcher {
    /// Create a new config watcher
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file to watch
    ///
    /// # Returns
    /// A ConfigWatcher instance that can be polled for reload events
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        // Track last event time for debouncing
        // We use a simple approach: ignore events within 100ms of each other
        let mut last_event: Option<Instant> = None;
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(100);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Only respond to modify events
                        match event.kind {
                            EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Any) => {
                                let now = Instant::now();

                                // Debounce: skip if last event was recent
                                if let Some(last) = last_event {
                                    if now.duration_since(last) < DEBOUNCE_DURATION {
                                        tracing::trace!("Config change debounced");
                                        return;
                                    }
                                }

                                last_event = Some(now);

                                tracing::info!("Config file changed, triggering reload");
                                let reload_event = ConfigReloadEvent { timestamp: now };

                                if let Err(e) = tx.send(reload_event) {
                                    tracing::error!(
                                        error = %e,
                                        "Failed to send config reload event"
                                    );
                                }
                            }
                            _ => {
                                // Ignore other event types (access, create, remove, etc.)
                                tracing::trace!(kind = ?event.kind, "Ignoring file event");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "File watcher error");
                    }
                }
            },
            NotifyConfig::default(),
        )
        .context("Failed to create file watcher")?;

        // Watch the config file
        watcher
            .watch(&config_path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch config file: {}", config_path.display()))?;

        tracing::info!(
            path = %config_path.display(),
            "Config file watcher initialized"
        );

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Try to receive a reload event (non-blocking)
    ///
    /// Returns Some(event) if a reload is pending, None otherwise.
    pub fn try_recv(&self) -> Option<ConfigReloadEvent> {
        self.receiver.try_recv().ok()
    }

    /// Get a reference to the receiver for calloop integration
    pub fn receiver(&self) -> &mpsc::Receiver<ConfigReloadEvent> {
        &self.receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_watcher_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test content").unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_path_buf();
        let watcher = ConfigWatcher::new(path);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_config_watcher_detects_changes() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "initial content").unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path().to_path_buf();
        let watcher = ConfigWatcher::new(path.clone()).unwrap();

        // Modify the file
        writeln!(temp_file, "modified content").unwrap();
        temp_file.flush().unwrap();

        // Give the watcher time to detect the change
        std::thread::sleep(Duration::from_millis(200));

        // Check for reload event
        let event = watcher.try_recv();
        assert!(event.is_some(), "Expected reload event after file modification");
    }
}
