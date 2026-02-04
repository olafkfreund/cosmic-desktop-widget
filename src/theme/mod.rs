//! Theming system for widget appearance

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

    /// Primary text color
    pub text_primary: Color,

    /// Secondary text color
    pub text_secondary: Color,

    /// Accent color (for decorations)
    pub accent: Color,

    /// Background transparency (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// Border width in pixels
    pub border_width: f32,

    /// Corner radius for rounded corners
    pub corner_radius: f32,
}

impl Theme {
    /// COSMIC-inspired dark theme
    pub fn cosmic_dark() -> Self {
        Self {
            background: Color::new(30, 30, 30, 230),
            border: Color::new(100, 100, 100, 255),
            text_primary: Color::new(255, 255, 255, 255),
            text_secondary: Color::new(180, 180, 180, 255),
            accent: Color::new(52, 120, 246, 255), // COSMIC blue
            opacity: 0.9,
            border_width: 2.0,
            corner_radius: 8.0,
        }
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            background: Color::new(255, 255, 255, 240),
            border: Color::new(200, 200, 200, 255),
            text_primary: Color::new(30, 30, 30, 255),
            text_secondary: Color::new(80, 80, 80, 255),
            accent: Color::new(52, 120, 246, 255),
            opacity: 0.95,
            border_width: 1.0,
            corner_radius: 8.0,
        }
    }

    /// Transparent dark theme
    pub fn transparent_dark() -> Self {
        Self {
            background: Color::new(0, 0, 0, 128),
            border: Color::new(255, 255, 255, 50),
            text_primary: Color::new(255, 255, 255, 255),
            text_secondary: Color::new(200, 200, 200, 200),
            accent: Color::new(52, 120, 246, 200),
            opacity: 0.5,
            border_width: 1.0,
            corner_radius: 12.0,
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
        let theme = Theme::transparent_dark();
        let bg = theme.background_with_opacity();
        assert!(bg.a < theme.background.a);
    }

    #[test]
    fn test_theme_from_name() {
        let dark = Theme::from_name("cosmic_dark");
        assert_eq!(dark.accent.r, 52);

        let light = Theme::from_name("light");
        assert_eq!(light.background.r, 255);

        let transparent = Theme::from_name("transparent_dark");
        assert_eq!(transparent.opacity, 0.5);

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
}
