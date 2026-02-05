//! Icon support for widgets
//!
//! Provides icon loading, caching, and rendering for widget content.
//! Supports both SVG and PNG formats with embedded common icons.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tiny_skia::{Pixmap, PixmapMut};
use tracing::debug;
use usvg::TreeParsing;

/// Icon loading and rendering errors
#[derive(Debug, Error)]
pub enum IconError {
    #[error("Failed to parse SVG: {0}")]
    SvgParse(String),

    #[error("Failed to render SVG: {0}")]
    SvgRender(String),

    #[error("Failed to load PNG: {0}")]
    PngLoad(String),

    #[error("Icon not found: {0}")]
    NotFound(String),

    #[error("Invalid icon size: {0}")]
    InvalidSize(String),
}

pub type IconResult<T> = Result<T, IconError>;

/// Icon data source
#[derive(Debug, Clone)]
pub enum IconSource {
    /// Embedded SVG data
    EmbeddedSvg(String),
    /// Embedded PNG data (not implemented yet)
    EmbeddedPng,
    /// External file path
    File(String),
}

/// Rendered icon with cached pixmap
#[derive(Clone)]
pub struct Icon {
    /// Cached pixmap at specific size
    pixmap: Arc<Pixmap>,
    /// Original source for re-rendering at different sizes
    source: IconSource,
}

impl Icon {
    /// Create icon from SVG data
    pub fn from_svg(svg_data: &str, size: u32) -> IconResult<Self> {
        let pixmap = Self::render_svg(svg_data, size)?;
        Ok(Self {
            pixmap: Arc::new(pixmap),
            source: IconSource::EmbeddedSvg(svg_data.to_string()),
        })
    }

    /// Create icon from PNG data
    pub fn from_png(png_data: &[u8]) -> IconResult<Self> {
        let pixmap = Self::load_png(png_data)?;
        Ok(Self {
            pixmap: Arc::new(pixmap),
            source: IconSource::EmbeddedPng,
        })
    }

    /// Get the cached pixmap
    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }

    /// Re-render at a different size
    pub fn resize(&self, size: u32) -> IconResult<Self> {
        match &self.source {
            IconSource::EmbeddedSvg(svg_data) => Self::from_svg(svg_data, size),
            IconSource::EmbeddedPng => {
                // For PNG, we'll scale the existing pixmap
                let new_pixmap = Self::scale_pixmap(&self.pixmap, size)?;
                Ok(Self {
                    pixmap: Arc::new(new_pixmap),
                    source: self.source.clone(),
                })
            }
            IconSource::File(_path) => {
                // TODO: Implement file loading
                Err(IconError::InvalidSize(
                    "File loading not implemented".to_string(),
                ))
            }
        }
    }

    /// Render SVG to pixmap
    fn render_svg(svg_data: &str, size: u32) -> IconResult<Pixmap> {
        debug!(size = size, "Rendering SVG icon");

        // Parse SVG
        let opt = usvg::Options::default();
        let svg_bytes = svg_data.as_bytes();
        let usvg_tree = usvg::Tree::from_data(svg_bytes, &opt)
            .map_err(|e| IconError::SvgParse(e.to_string()))?;

        // Convert to resvg tree
        let tree = resvg::Tree::from_usvg(&usvg_tree);

        // Create pixmap for rendering
        let mut pixmap = Pixmap::new(size, size)
            .ok_or_else(|| IconError::SvgRender("Failed to create pixmap".to_string()))?;

        // Calculate scaling transform
        let scale_x = size as f32 / tree.size.width();
        let scale_y = size as f32 / tree.size.height();
        let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

        // Render SVG to pixmap
        tree.render(transform, &mut pixmap.as_mut());

        Ok(pixmap)
    }

    /// Load PNG from bytes
    fn load_png(png_data: &[u8]) -> IconResult<Pixmap> {
        debug!(size = png_data.len(), "Loading PNG icon");

        let img =
            image::load_from_memory(png_data).map_err(|e| IconError::PngLoad(e.to_string()))?;

        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());

        // Convert to tiny-skia pixmap
        let mut pixmap = Pixmap::new(width, height)
            .ok_or_else(|| IconError::PngLoad("Failed to create pixmap".to_string()))?;

        // Copy pixel data (convert RGBA to premultiplied ARGB)
        for (i, chunk) in rgba.chunks_exact(4).enumerate() {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            // Premultiply alpha
            let a_f = a as f32 / 255.0;
            let r_pre = (r as f32 * a_f) as u8;
            let g_pre = (g as f32 * a_f) as u8;
            let b_pre = (b as f32 * a_f) as u8;

            pixmap.data_mut()[i * 4] = b_pre;
            pixmap.data_mut()[i * 4 + 1] = g_pre;
            pixmap.data_mut()[i * 4 + 2] = r_pre;
            pixmap.data_mut()[i * 4 + 3] = a;
        }

        Ok(pixmap)
    }

    /// Scale pixmap to new size (simple nearest-neighbor for now)
    fn scale_pixmap(pixmap: &Pixmap, new_size: u32) -> IconResult<Pixmap> {
        let mut new_pixmap = Pixmap::new(new_size, new_size)
            .ok_or_else(|| IconError::InvalidSize("Failed to create scaled pixmap".to_string()))?;

        let scale_x = pixmap.width() as f32 / new_size as f32;
        let scale_y = pixmap.height() as f32 / new_size as f32;

        for y in 0..new_size {
            for x in 0..new_size {
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;

                let src_idx = (src_y * pixmap.width() + src_x) as usize * 4;
                let dst_idx = (y * new_size + x) as usize * 4;

                if src_idx + 3 < pixmap.data().len() && dst_idx + 3 < new_pixmap.data().len() {
                    new_pixmap.data_mut()[dst_idx..dst_idx + 4]
                        .copy_from_slice(&pixmap.data()[src_idx..src_idx + 4]);
                }
            }
        }

        Ok(new_pixmap)
    }

    /// Draw icon onto a canvas at specified position
    pub fn draw(&self, canvas: &mut PixmapMut, x: i32, y: i32) {
        // Simple pixel copy with alpha blending
        let icon_width = self.pixmap.width() as i32;
        let icon_height = self.pixmap.height() as i32;
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;

        for icon_y in 0..icon_height {
            for icon_x in 0..icon_width {
                let canvas_x = x + icon_x;
                let canvas_y = y + icon_y;

                // Skip if outside canvas bounds
                if canvas_x < 0
                    || canvas_x >= canvas_width
                    || canvas_y < 0
                    || canvas_y >= canvas_height
                {
                    continue;
                }

                let icon_idx = (icon_y * icon_width + icon_x) as usize * 4;
                let canvas_idx = (canvas_y * canvas_width + canvas_x) as usize * 4;

                // Get icon pixel (premultiplied ARGB)
                let icon_data = self.pixmap.data();
                let b = icon_data[icon_idx];
                let g = icon_data[icon_idx + 1];
                let r = icon_data[icon_idx + 2];
                let a = icon_data[icon_idx + 3];

                if a == 255 {
                    // Fully opaque, direct copy
                    canvas.data_mut()[canvas_idx] = b;
                    canvas.data_mut()[canvas_idx + 1] = g;
                    canvas.data_mut()[canvas_idx + 2] = r;
                    canvas.data_mut()[canvas_idx + 3] = a;
                } else if a > 0 {
                    // Alpha blend with existing pixel
                    let a_f = a as f32 / 255.0;
                    let inv_a = 1.0 - a_f;

                    let canvas_data = canvas.data_mut();
                    let bg_b = canvas_data[canvas_idx] as f32;
                    let bg_g = canvas_data[canvas_idx + 1] as f32;
                    let bg_r = canvas_data[canvas_idx + 2] as f32;
                    let bg_a = canvas_data[canvas_idx + 3] as f32;

                    canvas_data[canvas_idx] = (b as f32 + bg_b * inv_a) as u8;
                    canvas_data[canvas_idx + 1] = (g as f32 + bg_g * inv_a) as u8;
                    canvas_data[canvas_idx + 2] = (r as f32 + bg_r * inv_a) as u8;
                    canvas_data[canvas_idx + 3] = (a as f32 + bg_a * inv_a).min(255.0) as u8;
                }
            }
        }
    }
}

/// Icon cache for efficient reuse
pub struct IconCache {
    cache: Mutex<HashMap<(String, u32), Arc<Icon>>>,
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create icon at specified size
    pub fn get_or_create(&self, name: &str, size: u32) -> IconResult<Arc<Icon>> {
        let key = (name.to_string(), size);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(icon) = cache.get(&key) {
                debug!(name = name, size = size, "Icon cache hit");
                return Ok(Arc::clone(icon));
            }
        }

        debug!(name = name, size = size, "Icon cache miss, loading");

        // Load icon from embedded data
        let icon = Self::load_embedded(name, size)?;

        // Cache for future use
        let icon_arc = Arc::new(icon);
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key, Arc::clone(&icon_arc));
        }

        Ok(icon_arc)
    }

    /// Load embedded icon by name
    fn load_embedded(name: &str, size: u32) -> IconResult<Icon> {
        match name {
            // Weather icons
            "weather-clear" => Icon::from_svg(ICON_WEATHER_CLEAR, size),
            "weather-clouds" => Icon::from_svg(ICON_WEATHER_CLOUDS, size),
            "weather-rain" => Icon::from_svg(ICON_WEATHER_RAIN, size),
            "weather-snow" => Icon::from_svg(ICON_WEATHER_SNOW, size),
            "weather-storm" => Icon::from_svg(ICON_WEATHER_STORM, size),

            // Battery icons
            "battery-full" => Icon::from_svg(ICON_BATTERY_FULL, size),
            "battery-charging" => Icon::from_svg(ICON_BATTERY_CHARGING, size),
            "battery-low" => Icon::from_svg(ICON_BATTERY_LOW, size),

            // MPRIS icons
            "media-play" => Icon::from_svg(ICON_MEDIA_PLAY, size),
            "media-pause" => Icon::from_svg(ICON_MEDIA_PAUSE, size),
            "media-next" => Icon::from_svg(ICON_MEDIA_NEXT, size),
            "media-previous" => Icon::from_svg(ICON_MEDIA_PREVIOUS, size),

            _ => Err(IconError::NotFound(name.to_string())),
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        debug!("Icon cache cleared");
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

// Embedded SVG icons (simple, minimal designs)
// Weather icons (based on common weather symbols)

const ICON_WEATHER_CLEAR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="5" fill="currentColor"/>
  <line x1="12" y1="1" x2="12" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="12" y1="21" x2="12" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="1" y1="12" x2="3" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="21" y1="12" x2="23" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
</svg>"#;

const ICON_WEATHER_CLOUDS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" fill="currentColor"/>
</svg>"#;

const ICON_WEATHER_RAIN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" fill="currentColor"/>
  <line x1="8" y1="19" x2="8" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="12" y1="19" x2="12" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <line x1="16" y1="19" x2="16" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
</svg>"#;

const ICON_WEATHER_SNOW: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" fill="currentColor"/>
  <circle cx="8" cy="21" r="1" fill="currentColor"/>
  <circle cx="12" cy="23" r="1" fill="currentColor"/>
  <circle cx="16" cy="21" r="1" fill="currentColor"/>
</svg>"#;

const ICON_WEATHER_STORM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" fill="currentColor"/>
  <path d="M13 13l-2 4h3l-2 4" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#;

// Battery icons
const ICON_BATTERY_FULL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <rect x="2" y="6" width="18" height="12" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
  <line x1="22" y1="10" x2="22" y2="14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <rect x="6" y="9" width="10" height="6" fill="currentColor"/>
</svg>"#;

const ICON_BATTERY_CHARGING: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <rect x="2" y="6" width="18" height="12" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
  <line x1="22" y1="10" x2="22" y2="14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <path d="M13 8l-3 5h3l-3 5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#;

const ICON_BATTERY_LOW: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <rect x="2" y="6" width="18" height="12" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
  <line x1="22" y1="10" x2="22" y2="14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
  <rect x="6" y="9" width="3" height="6" fill="currentColor"/>
</svg>"#;

// MPRIS media icons
const ICON_MEDIA_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" fill="none"/>
  <polygon points="10,8 16,12 10,16" fill="currentColor"/>
</svg>"#;

const ICON_MEDIA_PAUSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" fill="none"/>
  <rect x="9" y="8" width="2" height="8" fill="currentColor"/>
  <rect x="13" y="8" width="2" height="8" fill="currentColor"/>
</svg>"#;

const ICON_MEDIA_NEXT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <polygon points="6,6 12,12 6,18" fill="currentColor"/>
  <polygon points="12,6 18,12 12,18" fill="currentColor"/>
</svg>"#;

const ICON_MEDIA_PREVIOUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <polygon points="18,6 12,12 18,18" fill="currentColor"/>
  <polygon points="12,6 6,12 12,18" fill="currentColor"/>
</svg>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_from_svg() {
        let icon = Icon::from_svg(ICON_WEATHER_CLEAR, 24);
        assert!(icon.is_ok());
        let icon = icon.unwrap();
        assert_eq!(icon.pixmap().width(), 24);
        assert_eq!(icon.pixmap().height(), 24);
    }

    #[test]
    fn test_icon_cache() {
        let cache = IconCache::new();

        // First load
        let icon1 = cache.get_or_create("weather-clear", 24);
        assert!(icon1.is_ok());

        // Second load (should hit cache)
        let icon2 = cache.get_or_create("weather-clear", 24);
        assert!(icon2.is_ok());

        // Different size (should miss cache)
        let icon3 = cache.get_or_create("weather-clear", 32);
        assert!(icon3.is_ok());
    }

    #[test]
    fn test_icon_not_found() {
        let cache = IconCache::new();
        let result = cache.get_or_create("nonexistent-icon", 24);
        assert!(result.is_err());
        match result {
            Err(IconError::NotFound(name)) => assert_eq!(name, "nonexistent-icon"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_all_embedded_icons() {
        let cache = IconCache::new();
        let icon_names = vec![
            "weather-clear",
            "weather-clouds",
            "weather-rain",
            "weather-snow",
            "weather-storm",
            "battery-full",
            "battery-charging",
            "battery-low",
            "media-play",
            "media-pause",
            "media-next",
            "media-previous",
        ];

        for name in icon_names {
            let result = cache.get_or_create(name, 24);
            assert!(result.is_ok(), "Failed to load icon: {}", name);
        }
    }

    #[test]
    fn test_icon_resize() {
        let icon = Icon::from_svg(ICON_WEATHER_CLEAR, 24).unwrap();
        assert_eq!(icon.pixmap().width(), 24);

        let resized = icon.resize(48);
        assert!(resized.is_ok());
        let resized = resized.unwrap();
        assert_eq!(resized.pixmap().width(), 48);
    }
}
