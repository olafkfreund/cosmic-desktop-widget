//! Theming system for widget appearance
//!
//! Design tokens aligned with COSMIC Desktop guidelines:
//! - 8px spacing grid
//! - Semantic color palette
//! - Proper contrast ratios (WCAG AA minimum)
//! - Glassmorphic background patterns

use serde::{Deserialize, Serialize};

/// RGBA color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to tiny-skia Color
    pub fn to_tiny_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

/// Widget theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Background color
    pub background: Color,

    /// Border color
    pub border: Color,

    /// Primary text color (for data values - WCAG AAA: 7:1 contrast)
    pub text_primary: Color,

    /// Secondary text color (for labels - WCAG AA: 4.5:1 contrast)
    pub text_secondary: Color,

    /// Accent color (for decorations, progress bars)
    pub accent: Color,

    /// Background transparency (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// Enable compositor blur hint (for Wayland compositors that support it)
    #[serde(default)]
    pub blur_enabled: bool,

    /// Border width in pixels
    pub border_width: f32,

    /// Corner radius for rounded corners
    pub corner_radius: f32,

    /// Text shadow color (for readability on any wallpaper)
    #[serde(default = "default_text_shadow")]
    pub text_shadow: Color,

    /// Text shadow enabled
    #[serde(default = "default_shadow_enabled")]
    pub text_shadow_enabled: bool,
}

fn default_text_shadow() -> Color {
    Color::new(0, 0, 0, 153) // rgba(0,0,0,0.6)
}

fn default_shadow_enabled() -> bool {
    true
}

impl Theme {
    /// COSMIC-inspired dark theme - primary recommended theme
    ///
    /// Based on COSMIC Desktop design guidelines:
    /// - Dark background with 85% effective opacity for guaranteed readability
    /// - Subtle border (10% white) for definition against wallpaper
    /// - COSMIC blue accent (#3478F6)
    /// - 12px corner radius (COSMIC standard for containers)
    pub fn cosmic_dark() -> Self {
        Self {
            // Dark base with high alpha for readability on any wallpaper
            background: Color::new(20, 20, 24, 217), // ~85% opacity
            // Subtle light border for definition
            border: Color::new(255, 255, 255, 25), // 10% white
            // High contrast text - 95% white for data values
            text_primary: Color::new(255, 255, 255, 242),
            // Secondary text at 70% white for labels
            text_secondary: Color::new(255, 255, 255, 178),
            accent: Color::new(52, 120, 246, 255), // COSMIC blue
            opacity: 1.0,
            blur_enabled: false,
            border_width: 1.0,
            corner_radius: 12.0,
            text_shadow: Color::new(0, 0, 0, 128),
            text_shadow_enabled: true,
        }
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            background: Color::new(248, 248, 250, 217), // ~85% opacity
            border: Color::new(0, 0, 0, 20), // 8% black
            text_primary: Color::new(0, 0, 0, 230), // 90% black
            text_secondary: Color::new(0, 0, 0, 166), // 65% black
            accent: Color::new(52, 120, 246, 255),
            opacity: 1.0,
            blur_enabled: false,
            border_width: 1.0,
            corner_radius: 12.0,
            text_shadow: Color::new(255, 255, 255, 100),
            text_shadow_enabled: false,
        }
    }

    /// Transparent dark theme (glassmorphic without blur)
    pub fn transparent_dark() -> Self {
        Self {
            // 75% opacity dark background
            background: Color::new(20, 20, 24, 191),
            border: Color::new(255, 255, 255, 25),
            text_primary: Color::new(255, 255, 255, 242),
            text_secondary: Color::new(255, 255, 255, 178),
            accent: Color::new(52, 120, 246, 230),
            opacity: 1.0,
            blur_enabled: false,
            border_width: 1.0,
            corner_radius: 12.0,
            text_shadow: Color::new(0, 0, 0, 153),
            text_shadow_enabled: true,
        }
    }

    /// Transparent light theme (glassmorphic without blur)
    pub fn transparent_light() -> Self {
        Self {
            background: Color::new(248, 248, 250, 191), // 75% opacity
            border: Color::new(0, 0, 0, 20),
            text_primary: Color::new(0, 0, 0, 230),
            text_secondary: Color::new(0, 0, 0, 166),
            accent: Color::new(52, 120, 246, 230),
            opacity: 1.0,
            blur_enabled: false,
            border_width: 1.0,
            corner_radius: 12.0,
            text_shadow: Color::new(255, 255, 255, 100),
            text_shadow_enabled: false,
        }
    }

    /// Glass theme (frosted glass with blur hint)
    ///
    /// Uses lower background opacity because compositor blur
    /// provides additional contrast. Falls back gracefully if
    /// blur is not supported.
    pub fn glass() -> Self {
        Self {
            // Lower opacity because blur helps with contrast
            background: Color::new(20, 20, 24, 166), // ~65% opacity
            border: Color::new(255, 255, 255, 38), // 15% white - more visible with blur
            text_primary: Color::new(255, 255, 255, 242),
            text_secondary: Color::new(255, 255, 255, 178),
            accent: Color::new(52, 120, 246, 230),
            opacity: 1.0,
            blur_enabled: true,
            border_width: 1.0,
            corner_radius: 16.0,
            text_shadow: Color::new(0, 0, 0, 153),
            text_shadow_enabled: true,
        }
    }

    /// Get background color with opacity applied
    pub fn background_with_opacity(&self) -> Color {
        Color::new(
            self.background.r,
            self.background.g,
            self.background.b,
            (self.background.a as f32 * self.opacity) as u8,
        )
    }

    /// Load theme by name
    pub fn from_name(name: &str) -> Self {
        match name {
            "cosmic_dark" => Self::cosmic_dark(),
            "light" => Self::light(),
            "transparent_dark" => Self::transparent_dark(),
            "transparent_light" => Self::transparent_light(),
            "glass" => Self::glass(),
            _ => Self::cosmic_dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::cosmic_dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let color = Color::new(255, 128, 64, 200);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 200);
    }

    #[test]
    fn test_color_with_alpha() {
        let color = Color::rgb(255, 255, 255).with_alpha(128);
        assert_eq!(color.a, 128);
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme.accent.r, 52); // COSMIC blue
    }

    #[test]
    fn test_theme_opacity() {
        let theme = Theme::glass();
        let bg = theme.background_with_opacity();
        // Glass theme has opacity 1.0 so bg.a should equal background.a
        assert_eq!(bg.a, theme.background.a);
    }

    #[test]
    fn test_theme_from_name() {
        let dark = Theme::from_name("cosmic_dark");
        assert_eq!(dark.accent.r, 52);
        assert!(!dark.blur_enabled);

        let light = Theme::from_name("light");
        assert_eq!(light.background.r, 248);
        assert!(!light.blur_enabled);

        let transparent_dark = Theme::from_name("transparent_dark");
        assert!(!transparent_dark.blur_enabled);

        let transparent_light = Theme::from_name("transparent_light");
        assert_eq!(transparent_light.text_primary.r, 0); // Dark text
        assert!(!transparent_light.blur_enabled);

        let glass = Theme::from_name("glass");
        assert!(glass.blur_enabled);

        // Unknown theme defaults to cosmic_dark
        let unknown = Theme::from_name("unknown");
        assert_eq!(unknown.accent.r, 52);
    }

    #[test]
    fn test_color_to_array() {
        let color = Color::new(255, 128, 64, 200);
        let array = color.to_array();
        assert_eq!(array, [255, 128, 64, 200]);
    }

    #[test]
    fn test_text_shadow_defaults() {
        let theme = Theme::cosmic_dark();
        assert!(theme.text_shadow_enabled);
        assert!(theme.text_shadow.a > 0);
    }
}
