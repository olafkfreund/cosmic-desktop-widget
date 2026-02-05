// Font loading and management

use fontdue::{Font, FontSettings};
use std::sync::Arc;
use tracing::{debug, warn};

pub struct FontManager {
    font: Arc<Font>,
}

impl FontManager {
    pub fn new() -> Self {
        // Try to find fonts using multiple strategies

        // Strategy 1: Use fontconfig via fc-list to find fonts dynamically
        if let Some(manager) = Self::try_fontconfig() {
            return manager;
        }

        // Strategy 2: Try well-known paths
        if let Some(manager) = Self::try_known_paths() {
            return manager;
        }

        // Strategy 3: Search common font directories
        if let Some(manager) = Self::try_search_dirs() {
            return manager;
        }

        panic!("No usable font found. Please install DejaVu Sans or Liberation Sans fonts.");
    }

    fn try_fontconfig() -> Option<Self> {
        // Try to use fc-match to find a suitable font
        use std::process::Command;

        let output = Command::new("fc-match")
            .args(["--format=%{file}", "sans"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let path = String::from_utf8(output.stdout).ok()?;
        let path = path.trim();

        if path.is_empty() {
            return None;
        }

        Self::try_load_font(path)
    }

    fn try_known_paths() -> Option<Self> {
        // Standard paths across different Linux distributions
        let font_paths = [
            // Standard Linux paths
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/gnu-free/FreeSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];

        for path in &font_paths {
            if let Some(manager) = Self::try_load_font(path) {
                return Some(manager);
            }
        }
        None
    }

    fn try_search_dirs() -> Option<Self> {
        // Search common font directories
        let search_dirs = [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/nix/var/nix/profiles/default/share/fonts",
        ];

        // Also check XDG_DATA_DIRS for fonts
        if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in xdg_dirs.split(':') {
                let font_dir = format!("{}/fonts", dir);
                if let Some(manager) = Self::search_dir_for_font(&font_dir) {
                    return Some(manager);
                }
            }
        }

        for dir in &search_dirs {
            if let Some(manager) = Self::search_dir_for_font(dir) {
                return Some(manager);
            }
        }
        None
    }

    fn search_dir_for_font(dir: &str) -> Option<Self> {
        use std::fs;

        let path = std::path::Path::new(dir);
        if !path.exists() {
            return None;
        }

        // Look for common sans-serif fonts
        let font_names = [
            "DejaVuSans.ttf",
            "NotoSans-Regular.ttf",
            "LiberationSans-Regular.ttf",
        ];

        fn visit_dirs(dir: &std::path::Path, font_names: &[&str]) -> Option<String> {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(found) = visit_dirs(&path, font_names) {
                            return Some(found);
                        }
                    } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if font_names.contains(&name) {
                            return Some(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
            None
        }

        if let Some(font_path) = visit_dirs(path, &font_names) {
            return Self::try_load_font(&font_path);
        }
        None
    }

    fn try_load_font(path: &str) -> Option<Self> {
        let font_data = std::fs::read(path).ok()?;
        match Font::from_bytes(font_data, FontSettings::default()) {
            Ok(font) => {
                debug!("Loaded font from: {}", path);
                Some(Self {
                    font: Arc::new(font),
                })
            }
            Err(e) => {
                warn!("Failed to parse font at {}: {}", path, e);
                None
            }
        }
    }

    pub fn font(&self) -> &Font {
        &self.font
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_manager_creation() {
        let manager = FontManager::new();
        let font = manager.font();
        // Just verify we got a font
        assert!(font.horizontal_line_metrics(16.0).is_some());
    }
}
