// Text rendering with alpha blending
//
// Performance optimizations:
// - Avoid cloning glyph bitmaps by using free function for blitting
// - Reduced allocations in hot path
// - Efficient alpha blending

use super::{FontManager, FontWeight, GlyphCache};
use tiny_skia::PixmapMut;
use tracing::trace;

pub struct TextRenderer {
    font_manager: FontManager,
    glyph_cache: GlyphCache,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font_manager: FontManager::new(),
            glyph_cache: GlyphCache::new(),
        }
    }

    /// Render text with default (regular) weight
    pub fn render_text(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4], // RGBA
    ) {
        self.render_text_weighted(pixmap, text, x, y, size, color, FontWeight::Regular);
    }

    /// Render text with specified font weight
    ///
    /// The `y` parameter is treated as the baseline position.
    /// Glyphs are placed using their individual xmin/ymin offsets from fontdue
    /// for correct character positioning.
    pub fn render_text_weighted(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 4], // RGBA
        weight: FontWeight,
    ) {
        trace!(
            "Rendering text: '{}' at ({}, {}) size {} weight {:?}",
            text, x, y, size, weight
        );

        self.glyph_cache.clear_if_full();

        let mut cursor_x = x;
        let font = self.font_manager.font(weight);

        // Use font's actual line metrics for proper baseline
        let baseline_y = y as i32;

        for c in text.chars() {
            // Get glyph from cache (no cloning - use borrowed reference)
            let glyph = self.glyph_cache.get_or_rasterize(font, c, size, weight);

            // Calculate correct glyph position using fontdue metrics:
            // - xmin: horizontal offset from cursor to glyph bitmap left edge
            // - ymin: vertical offset from baseline to glyph bitmap bottom edge
            // In screen coords (y-down), the glyph top is at:
            //   baseline_y - ymin - height
            let glyph_x = cursor_x as i32 + glyph.xmin;
            let glyph_y = baseline_y - glyph.ymin - glyph.height as i32;

            // Blit the glyph bitmap to the pixmap with alpha blending
            blit_glyph(
                pixmap,
                &glyph.bitmap,
                glyph.width,
                glyph.height,
                glyph_x,
                glyph_y,
                color,
            );

            cursor_x += glyph.advance_width;
        }
    }
}

/// Blit a glyph bitmap onto a pixmap with alpha blending
///
/// This is a free function to avoid borrow conflicts when iterating
/// through the glyph cache while rendering.
#[allow(clippy::too_many_arguments)]
#[inline]
fn blit_glyph(
    pixmap: &mut PixmapMut,
    bitmap: &[u8],
    glyph_width: usize,
    glyph_height: usize,
    x: i32,
    y: i32,
    color: [u8; 4],
) {
    let pixmap_width = pixmap.width() as i32;
    let pixmap_height = pixmap.height() as i32;
    let pixels = pixmap.pixels_mut();

    // Pre-compute color components for blending
    let color_r = color[0] as f32;
    let color_g = color[1] as f32;
    let color_b = color[2] as f32;
    let color_a_factor = color[3] as f32 / 255.0;

    for gy in 0..glyph_height {
        let py = y + gy as i32;

        // Skip entire row if out of bounds
        if py < 0 || py >= pixmap_height {
            continue;
        }

        let row_start = py * pixmap_width;

        for gx in 0..glyph_width {
            let px = x + gx as i32;

            // Skip if out of bounds horizontally
            if px < 0 || px >= pixmap_width {
                continue;
            }

            let glyph_alpha = bitmap[gy * glyph_width + gx];
            if glyph_alpha == 0 {
                continue;
            }

            let idx = (row_start + px) as usize;
            let pixel = &mut pixels[idx];

            // Alpha blend with tiny-skia's premultiplied alpha
            let alpha = (glyph_alpha as f32 / 255.0) * color_a_factor;
            let inv_alpha = 1.0 - alpha;

            // Demultiply existing pixel
            let dst = pixel.demultiply();

            // Blend in linear space (using pre-computed color values)
            let new_r = (color_r * alpha + dst.red() as f32 * inv_alpha).clamp(0.0, 255.0) as u8;
            let new_g = (color_g * alpha + dst.green() as f32 * inv_alpha).clamp(0.0, 255.0) as u8;
            let new_b = (color_b * alpha + dst.blue() as f32 * inv_alpha).clamp(0.0, 255.0) as u8;
            let new_a =
                ((alpha + dst.alpha() as f32 / 255.0 * inv_alpha) * 255.0).clamp(0.0, 255.0) as u8;

            // Create premultiplied color for tiny-skia
            *pixel = tiny_skia::PremultipliedColorU8::from_rgba(new_r, new_g, new_b, new_a)
                .unwrap_or(*pixel); // Fallback to original pixel if color creation fails
        }
    }
}

impl TextRenderer {
    /// Calculate text width for layout purposes (uses regular weight)
    pub fn measure_text(&mut self, text: &str, size: f32) -> f32 {
        self.measure_text_weighted(text, size, FontWeight::Regular)
    }

    /// Calculate text width with specified font weight
    pub fn measure_text_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        let font = self.font_manager.font(weight);
        let mut width = 0.0;

        for c in text.chars() {
            let glyph = self.glyph_cache.get_or_rasterize(font, c, size, weight);
            width += glyph.advance_width;
        }

        width
    }

    /// Get the font ascent for a given size (distance from baseline to top of tallest glyph)
    pub fn ascent(&self, size: f32) -> f32 {
        let font = self.font_manager.font(FontWeight::Regular);
        if let Some(metrics) = font.horizontal_line_metrics(size) {
            metrics.ascent
        } else {
            size * 0.8 // Fallback approximation
        }
    }

    /// Get the font descent for a given size (distance from baseline to bottom of lowest glyph, typically negative)
    pub fn descent(&self, size: f32) -> f32 {
        let font = self.font_manager.font(FontWeight::Regular);
        if let Some(metrics) = font.horizontal_line_metrics(size) {
            metrics.descent
        } else {
            size * -0.2 // Fallback approximation
        }
    }

    /// Calculate the baseline Y position for vertically centering text
    /// within a region of the given height at the given y_center
    pub fn baseline_for_center(&self, size: f32, y_center: f32) -> f32 {
        let ascent = self.ascent(size);
        let descent = self.descent(size);
        // Center the text block (ascent + |descent|) around y_center
        y_center + (ascent + descent) / 2.0
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Pixmap;

    #[test]
    fn test_text_renderer_creation() {
        use super::super::FontWeight;
        let renderer = TextRenderer::new();
        // Just verify it creates without panic
        assert!(renderer
            .font_manager
            .font(FontWeight::Regular)
            .horizontal_line_metrics(16.0)
            .is_some());
    }

    #[test]
    fn test_measure_text() {
        let mut renderer = TextRenderer::new();
        let width = renderer.measure_text("Hello", 16.0);
        assert!(width > 0.0);

        // Longer text should be wider
        let width2 = renderer.measure_text("Hello World!", 16.0);
        assert!(width2 > width);
    }

    #[test]
    fn test_render_text() {
        let mut renderer = TextRenderer::new();
        let mut pixmap = Pixmap::new(200, 100).unwrap();
        let mut pixmap_mut = pixmap.as_mut();

        // Should not panic
        renderer.render_text(
            &mut pixmap_mut,
            "Test",
            10.0,
            50.0,
            16.0,
            [255, 255, 255, 255],
        );
    }
}
