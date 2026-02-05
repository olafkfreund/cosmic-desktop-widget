// Font loading and management

use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Font weight for text rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontWeight {
    /// Regular/normal weight
    #[default]
    Regular,
    /// Bold weight
    Bold,
}

pub struct FontManager {
    fonts: HashMap<FontWeight, Arc<Font>>,
}

impl FontManager {
    pub fn new() -> Self {
        let mut fonts = HashMap::new();

        // Try to find fonts using multiple strategies
        // Strategy 1: Use fontconfig via fc-list to find fonts dynamically
        if let Some((regular, bold)) = Self::try_fontconfig() {
            fonts.insert(FontWeight::Regular, regular);
            if let Some(bold_font) = bold {
                fonts.insert(FontWeight::Bold, bold_font);
            }
            return Self { fonts };
        }

        // Strategy 2: Try well-known paths
        if let Some((regular, bold)) = Self::try_known_paths() {
            fonts.insert(FontWeight::Regular, regular);
            if let Some(bold_font) = bold {
                fonts.insert(FontWeight::Bold, bold_font);
            }
            return Self { fonts };
        }

        // Strategy 3: Search common font directories
        if let Some((regular, bold)) = Self::try_search_dirs() {
            fonts.insert(FontWeight::Regular, regular);
            if let Some(bold_font) = bold {
                fonts.insert(FontWeight::Bold, bold_font);
            }
            return Self { fonts };
        }

        panic!("No usable font found. Please install DejaVu Sans or Liberation Sans fonts.");
    }

    fn try_fontconfig() -> Option<(Arc<Font>, Option<Arc<Font>>)> {
        // Try to use fc-match to find suitable fonts
        use std::process::Command;

        // Preferred fonts in order - modern, clean, readable
        let preferred_fonts = [
            "Fira Sans",      // Modern, clean, excellent readability
            "Inter",          // Very popular, highly readable
            "Roboto",         // Google's modern sans-serif
            "Adwaita Sans",   // GNOME default
            "Cantarell",      // GNOME alternative
            "Noto Sans",      // Google's universal font
            "DejaVu Sans",    // Fallback, widely available
            "Liberation Sans", // Another common fallback
        ];

        // Try each preferred font in order
        for font_family in &preferred_fonts {
            let regular_query = format!("{}:weight=regular", font_family);
            let output = Command::new("fc-match")
                .args(["--format=%{file}", &regular_query])
                .output()
                .ok();

            if let Some(output) = output {
                if output.status.success() {
                    let regular_path = String::from_utf8(output.stdout).ok();
                    if let Some(regular_path) = regular_path {
                        let regular_path = regular_path.trim();
                        // Verify this is actually the font we asked for (not a fallback)
                        if !regular_path.is_empty() && regular_path.to_lowercase().contains(&font_family.to_lowercase().replace(' ', "")) {
                            if let Some(regular_font) = Self::load_font_file(regular_path) {
                                debug!("Loaded regular font: {} from {}", font_family, regular_path);

                                // Try to get matching bold font
                                let bold_query = format!("{}:weight=bold", font_family);
                                let bold_font = Command::new("fc-match")
                                    .args(["--format=%{file}", &bold_query])
                                    .output()
                                    .ok()
                                    .and_then(|output| {
                                        if output.status.success() {
                                            let bold_path = String::from_utf8(output.stdout).ok()?;
                                            let bold_path = bold_path.trim();
                                            if !bold_path.is_empty() && bold_path != regular_path {
                                                let font = Self::load_font_file(bold_path)?;
                                                debug!("Loaded bold font: {} from {}", font_family, bold_path);
                                                Some(font)
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    });

                                return Some((regular_font, bold_font));
                            }
                        }
                    }
                }
            }
        }

        // Fallback: just get any sans font
        let output = Command::new("fc-match")
            .args(["--format=%{file}", "sans:weight=regular"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let regular_path = String::from_utf8(output.stdout).ok()?;
        let regular_path = regular_path.trim();

        if regular_path.is_empty() {
            return None;
        }

        let regular_font = Self::load_font_file(regular_path)?;
        debug!("Loaded regular font from: {}", regular_path);

        // Try to get bold font
        let bold_font = Command::new("fc-match")
            .args(["--format=%{file}", "sans:weight=bold"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let bold_path = String::from_utf8(output.stdout).ok()?;
                    let bold_path = bold_path.trim();
                    if !bold_path.is_empty() && bold_path != regular_path {
                        let font = Self::load_font_file(bold_path)?;
                        debug!("Loaded bold font from: {}", bold_path);
                        Some(font)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        Some((regular_font, bold_font))
    }

    fn try_known_paths() -> Option<(Arc<Font>, Option<Arc<Font>>)> {
        // Standard paths across different Linux distributions
        let regular_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/gnu-free/FreeSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];

        let bold_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/noto/NotoSans-Bold.ttf",
            "/usr/share/fonts/liberation/LiberationSans-Bold.ttf",
            "/usr/share/fonts/gnu-free/FreeSansBold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ];

        for (i, path) in regular_paths.iter().enumerate() {
            if let Some(regular_font) = Self::load_font_file(path) {
                debug!("Loaded regular font from: {}", path);
                let bold_font = bold_paths.get(i).and_then(|bold_path| {
                    let font = Self::load_font_file(bold_path)?;
                    debug!("Loaded bold font from: {}", bold_path);
                    Some(font)
                });
                return Some((regular_font, bold_font));
            }
        }
        None
    }

    fn try_search_dirs() -> Option<(Arc<Font>, Option<Arc<Font>>)> {
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
                if let Some(result) = Self::search_dir_for_fonts(&font_dir) {
                    return Some(result);
                }
            }
        }

        for dir in &search_dirs {
            if let Some(result) = Self::search_dir_for_fonts(dir) {
                return Some(result);
            }
        }
        None
    }

    fn search_dir_for_fonts(dir: &str) -> Option<(Arc<Font>, Option<Arc<Font>>)> {
        use std::fs;

        let path = std::path::Path::new(dir);
        if !path.exists() {
            return None;
        }

        // Look for common sans-serif fonts (regular and bold)
        let regular_font_names = [
            "DejaVuSans.ttf",
            "NotoSans-Regular.ttf",
            "LiberationSans-Regular.ttf",
        ];
        let bold_font_names = [
            "DejaVuSans-Bold.ttf",
            "NotoSans-Bold.ttf",
            "LiberationSans-Bold.ttf",
        ];

        fn visit_dirs(
            dir: &std::path::Path,
            font_names: &[&str],
        ) -> Option<String> {
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

        // Find regular font
        let regular_path = visit_dirs(path, &regular_font_names)?;
        let regular_font = Self::load_font_file(&regular_path)?;
        debug!("Loaded regular font from: {}", regular_path);

        // Try to find bold font
        let bold_font = visit_dirs(path, &bold_font_names).and_then(|bold_path| {
            let font = Self::load_font_file(&bold_path)?;
            debug!("Loaded bold font from: {}", bold_path);
            Some(font)
        });

        Some((regular_font, bold_font))
    }

    fn load_font_file(path: &str) -> Option<Arc<Font>> {
        let font_data = std::fs::read(path).ok()?;
        match Font::from_bytes(font_data, FontSettings::default()) {
            Ok(font) => Some(Arc::new(font)),
            Err(e) => {
                warn!("Failed to parse font at {}: {}", path, e);
                None
            }
        }
    }

    /// Get font by weight, falling back to regular if requested weight unavailable
    pub fn font(&self, weight: FontWeight) -> &Font {
        self.fonts
            .get(&weight)
            .or_else(|| self.fonts.get(&FontWeight::Regular))
            .expect("At least regular font must be loaded")
    }

    /// Get regular font (convenience method for backward compatibility)
    pub fn font_regular(&self) -> &Font {
        self.font(FontWeight::Regular)
    }

    /// Check if bold font is available
    pub fn has_bold(&self) -> bool {
        self.fonts.contains_key(&FontWeight::Bold)
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
        let font = manager.font(FontWeight::Regular);
        // Just verify we got a font
        assert!(font.horizontal_line_metrics(16.0).is_some());
    }

    #[test]
    fn test_font_weight_default() {
        assert_eq!(FontWeight::default(), FontWeight::Regular);
    }

    #[test]
    fn test_font_regular_convenience() {
        let manager = FontManager::new();
        let regular = manager.font_regular();
        let also_regular = manager.font(FontWeight::Regular);
        // Both should be valid fonts
        assert!(regular.horizontal_line_metrics(16.0).is_some());
        assert!(also_regular.horizontal_line_metrics(16.0).is_some());
    }

    #[test]
    fn test_bold_fallback() {
        let manager = FontManager::new();
        // Even if bold isn't available, requesting it should not panic
        let font = manager.font(FontWeight::Bold);
        assert!(font.horizontal_line_metrics(16.0).is_some());
    }
}
