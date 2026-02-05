// Rendering engine using tiny-skia
//
// Performance optimizations:
// - Dirty region tracking to avoid full redraws
// - Cached font size calculations
// - Cached text content to detect changes
// - Efficient partial updates

use crate::config::Config;
use crate::icons::IconCache;
use crate::text::TextRenderer;
use crate::theme::Theme;
use crate::widget::traits::Widget;
use crate::widget::{ClockWidget, WeatherWidget};
use chrono::Timelike;
use tiny_skia::*;
use tracing::{instrument, trace, warn};

/// Target width percentage for clock text (0.0-1.0)
const CLOCK_WIDTH_RATIO: f32 = 0.80;
/// Minimum and maximum font sizes
const MIN_FONT_SIZE: f32 = 24.0;
const MAX_FONT_SIZE: f32 = 144.0;
/// Weather font as ratio of clock font
const WEATHER_FONT_RATIO: f32 = 0.35;

/// Represents a rectangular region that needs redrawing
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl DirtyRegion {
    /// Create a new dirty region covering the entire area
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
            dirty: true,
        }
    }

    /// Mark region as clean (no redraw needed)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Mark region as dirty (needs redraw)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if region needs redrawing
    pub fn needs_redraw(&self) -> bool {
        self.dirty
    }
}

/// Cached render state to avoid redundant calculations
#[derive(Debug, Default)]
struct RenderCache {
    /// Last rendered clock text
    last_clock_text: String,
    /// Last rendered weather text
    last_weather_text: Option<String>,
    /// Cached clock font size
    clock_font_size: f32,
    /// Cached weather font size
    weather_font_size: f32,
    /// Last width used for font calculation
    last_width: u32,
    /// Last height used for font calculation
    last_height: u32,
    /// Whether weather was visible last frame
    last_had_weather: bool,
    /// Last seconds value for progress bar
    last_seconds: u32,
}

impl RenderCache {
    fn new() -> Self {
        Self::default()
    }

    /// Check if clock text changed
    fn clock_changed(&self, new_text: &str) -> bool {
        self.last_clock_text != new_text
    }

    /// Check if weather text changed
    fn weather_changed(&self, new_text: Option<&str>) -> bool {
        match (&self.last_weather_text, new_text) {
            (None, None) => false,
            (Some(old), Some(new)) => old != new,
            _ => true,
        }
    }

    /// Check if font sizes need recalculation
    fn needs_font_recalc(&self, width: u32, height: u32, has_weather: bool) -> bool {
        self.last_width != width
            || self.last_height != height
            || self.last_had_weather != has_weather
            || self.clock_font_size == 0.0
    }

    /// Update cached values
    fn update(
        &mut self,
        clock_text: &str,
        weather_text: Option<&str>,
        clock_font_size: f32,
        weather_font_size: f32,
        width: u32,
        height: u32,
        has_weather: bool,
        seconds: u32,
    ) {
        self.last_clock_text = clock_text.to_string();
        self.last_weather_text = weather_text.map(String::from);
        self.clock_font_size = clock_font_size;
        self.weather_font_size = weather_font_size;
        self.last_width = width;
        self.last_height = height;
        self.last_had_weather = has_weather;
        self.last_seconds = seconds;
    }
}

pub struct Renderer {
    text_renderer: TextRenderer,
    theme: Theme,
    /// Icon cache for efficient icon rendering
    icon_cache: IconCache,
    /// Dirty region tracking
    dirty_region: DirtyRegion,
    /// Render cache for optimization
    cache: RenderCache,
    /// Whether this is the first render (always do full draw)
    first_render: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            text_renderer: TextRenderer::new(),
            theme: Theme::default(),
            icon_cache: IconCache::new(),
            dirty_region: DirtyRegion::default(),
            cache: RenderCache::new(),
            first_render: true,
        }
    }

    pub fn with_theme(theme: Theme) -> Self {
        Self {
            text_renderer: TextRenderer::new(),
            theme,
            icon_cache: IconCache::new(),
            dirty_region: DirtyRegion::default(),
            cache: RenderCache::new(),
            first_render: true,
        }
    }

    /// Mark the entire surface as dirty (needs full redraw)
    pub fn mark_dirty(&mut self) {
        self.dirty_region.mark_dirty();
    }

    /// Check if any region needs redrawing
    pub fn needs_redraw(&self) -> bool {
        self.dirty_region.needs_redraw() || self.first_render
    }

    /// Get the dirty region for damage reporting
    pub fn dirty_region(&self) -> &DirtyRegion {
        &self.dirty_region
    }

    /// Check if content has changed and needs redrawing
    /// Returns (clock_changed, weather_changed, progress_changed)
    pub fn check_content_changes(
        &self,
        clock: Option<&ClockWidget>,
        weather: Option<&WeatherWidget>,
    ) -> (bool, bool, bool) {
        let clock_text = clock.map(|c| c.time_string());
        let weather_text = weather.and_then(|w| w.display_string());
        let seconds = chrono::Local::now().second();

        let clock_changed = match &clock_text {
            Some(text) => self.cache.clock_changed(text),
            None => !self.cache.last_clock_text.is_empty(),
        };

        let weather_changed = self.cache.weather_changed(weather_text.as_deref());
        let progress_changed = self.cache.last_seconds != seconds;

        (clock_changed, weather_changed, progress_changed)
    }

    #[instrument(skip(self, canvas, clock, weather, config), fields(width = %width, height = %height))]
    pub fn render(
        &mut self,
        canvas: &mut [u8],
        width: u32,
        height: u32,
        clock: Option<&ClockWidget>,
        weather: Option<&WeatherWidget>,
        config: &Config,
    ) {
        // Check what actually changed
        let (clock_changed, weather_changed, progress_changed) =
            self.check_content_changes(clock, weather);

        // Skip render if nothing changed (unless first render)
        if !self.first_render && !clock_changed && !weather_changed && !progress_changed {
            trace!("Skipping render - no changes detected");
            self.dirty_region.mark_clean();
            return;
        }

        trace!(
            clock_changed = clock_changed,
            weather_changed = weather_changed,
            progress_changed = progress_changed,
            first_render = self.first_render,
            "Starting render"
        );

        // Create pixmap from canvas
        let Some(mut pixmap) = PixmapMut::from_bytes(canvas, width, height) else {
            tracing::error!(
                width = width,
                height = height,
                canvas_len = canvas.len(),
                "Failed to create pixmap - invalid dimensions or buffer size"
            );
            return;
        };

        // Clear background with theme color
        let bg = self.theme.background_with_opacity();
        let bg_color = bg.to_tiny_skia();
        pixmap.fill(bg_color);

        // Draw rounded rectangle background with corner radius
        let corner_radius = self.theme.corner_radius;
        self.draw_rounded_rect(&mut pixmap, width, height, corner_radius, &bg);

        // Draw border with rounded corners
        self.draw_rounded_border(&mut pixmap, width, height, corner_radius);

        let width_f = width as f32;
        let height_f = height as f32;
        let padding = config.padding();

        // Calculate vertical layout
        let has_clock = clock.is_some();
        let has_weather = weather.is_some()
            && weather
                .as_ref()
                .map_or(false, |w| w.display_string().is_some());

        // Get current text values
        let clock_text = clock.map(|c| c.time_string());
        let weather_text = weather.and_then(|w| w.display_string());

        // Use cached font sizes if dimensions haven't changed
        let (clock_font_size, weather_font_size) =
            if self.cache.needs_font_recalc(width, height, has_weather) {
                let target_width = (width_f - padding * 2.0) * CLOCK_WIDTH_RATIO;
                let clock_size = if let Some(ref text) = clock_text {
                    self.calculate_font_size(text, target_width, has_weather)
                } else {
                    MIN_FONT_SIZE
                };
                let weather_size = (clock_size * WEATHER_FONT_RATIO).max(16.0);
                trace!(
                    clock_font_size = clock_size,
                    weather_font_size = weather_size,
                    "Recalculated font sizes"
                );
                (clock_size, weather_size)
            } else {
                (self.cache.clock_font_size, self.cache.weather_font_size)
            };

        // Render clock if enabled - centered
        if let Some(ref time_str) = clock_text {
            let text_width = self.text_renderer.measure_text(time_str, clock_font_size);

            // Center horizontally
            let x = (width_f - text_width) / 2.0;

            // Center vertically (accounting for both widgets if present)
            let y = if has_weather {
                // Clock in upper portion when weather is shown
                height_f * 0.42 + clock_font_size * 0.35
            } else {
                // Perfectly centered when alone
                height_f / 2.0 + clock_font_size * 0.35
            };

            self.render_text(&mut pixmap, time_str, x, y, clock_font_size);
        }

        // Render weather if enabled - centered below clock
        if let Some(ref weather_str) = weather_text {
            let text_width = self
                .text_renderer
                .measure_text(weather_str, weather_font_size);

            // Center horizontally
            let x = (width_f - text_width) / 2.0;

            // Position below clock or centered
            let y = if has_clock {
                height_f * 0.82 + weather_font_size * 0.35
            } else {
                height_f / 2.0 + weather_font_size * 0.35
            };

            self.render_text(&mut pixmap, weather_str, x, y, weather_font_size);
        }

        // Draw minute progress bar at bottom
        let seconds = chrono::Local::now().second();
        self.draw_minute_progress(&mut pixmap, width, height, padding, seconds);

        // Update cache with current values
        self.cache.update(
            clock_text.as_deref().unwrap_or(""),
            weather_text.as_deref(),
            clock_font_size,
            weather_font_size,
            width,
            height,
            has_weather,
            seconds,
        );

        // Mark dirty region
        self.dirty_region = DirtyRegion::full(width, height);
        self.first_render = false;

        trace!("Render complete");
    }

    /// Calculate optimal font size to fill the target width
    fn calculate_font_size(&mut self, text: &str, target_width: f32, has_weather: bool) -> f32 {
        // When showing weather, reduce max font size to leave room
        let max_size = if has_weather {
            MAX_FONT_SIZE * 0.70
        } else {
            MAX_FONT_SIZE
        };

        // Binary search for optimal font size
        let mut low = MIN_FONT_SIZE;
        let mut high = max_size;

        while high - low > 1.0 {
            let mid = (low + high) / 2.0;
            let width = self.text_renderer.measure_text(text, mid);

            if width < target_width {
                low = mid;
            } else {
                high = mid;
            }
        }

        // Use the lower bound to ensure we don't exceed target
        low.clamp(MIN_FONT_SIZE, max_size)
    }

    /// Draw a rounded rectangle background
    fn draw_rounded_rect(
        &self,
        pixmap: &mut PixmapMut,
        width: u32,
        height: u32,
        radius: f32,
        color: &crate::theme::Color,
    ) {
        let mut paint = Paint::default();
        let rgba = color.to_array();
        paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
        paint.anti_alias = true;

        let path = self.create_rounded_rect_path(width as f32, height as f32, radius);
        if let Some(path) = path {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Draw a rounded border
    fn draw_rounded_border(&self, pixmap: &mut PixmapMut, width: u32, height: u32, radius: f32) {
        let mut paint = Paint::default();
        let border = self.theme.border.to_array();
        paint.set_color_rgba8(border[0], border[1], border[2], border[3]);
        paint.anti_alias = true;

        let stroke = Stroke {
            width: self.theme.border_width,
            ..Default::default()
        };

        let path = self.create_rounded_rect_path(width as f32, height as f32, radius);
        if let Some(path) = path {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Create a path for a rounded rectangle
    fn create_rounded_rect_path(&self, width: f32, height: f32, radius: f32) -> Option<Path> {
        let r = radius.min(width / 2.0).min(height / 2.0);
        let mut pb = PathBuilder::new();

        // Start at top-left after the corner
        pb.move_to(r, 0.0);

        // Top edge and top-right corner
        pb.line_to(width - r, 0.0);
        pb.quad_to(width, 0.0, width, r);

        // Right edge and bottom-right corner
        pb.line_to(width, height - r);
        pb.quad_to(width, height, width - r, height);

        // Bottom edge and bottom-left corner
        pb.line_to(r, height);
        pb.quad_to(0.0, height, 0.0, height - r);

        // Left edge and top-left corner
        pb.line_to(0.0, r);
        pb.quad_to(0.0, 0.0, r, 0.0);

        pb.close();
        pb.finish()
    }

    /// Draw a minute progress bar at the bottom
    /// Shows progress through the current minute (0-59 seconds)
    fn draw_minute_progress(
        &self,
        pixmap: &mut PixmapMut,
        width: u32,
        height: u32,
        padding: f32,
        seconds: u32,
    ) {
        let y = height as f32 - padding * 0.6;
        let margin = padding * 1.5;
        let bar_height = 4.0;
        let total_width = width as f32 - margin * 2.0;

        // Calculate progress (0.0 to 1.0)
        let progress = seconds as f32 / 60.0;

        // Draw background track (dim)
        let mut bg_paint = Paint::default();
        let accent = self.theme.accent.to_array();
        bg_paint.set_color_rgba8(accent[0], accent[1], accent[2], 40); // Very dim
        bg_paint.anti_alias = true;

        if let Some(bg_rect) =
            Rect::from_xywh(margin, y - bar_height / 2.0, total_width, bar_height)
        {
            let bg_path = PathBuilder::from_rect(bg_rect);
            pixmap.fill_path(
                &bg_path,
                &bg_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        // Draw progress fill (bright accent)
        if progress > 0.0 {
            let mut fg_paint = Paint::default();
            fg_paint.set_color_rgba8(accent[0], accent[1], accent[2], accent[3]);
            fg_paint.anti_alias = true;

            let fill_width = total_width * progress;
            if let Some(fg_rect) =
                Rect::from_xywh(margin, y - bar_height / 2.0, fill_width, bar_height)
            {
                let fg_path = PathBuilder::from_rect(fg_rect);
                pixmap.fill_path(
                    &fg_path,
                    &fg_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    fn render_text(&mut self, pixmap: &mut PixmapMut, text: &str, x: f32, y: f32, size: f32) {
        // Render text using fontdue with theme color
        let text_color = self.theme.text_primary.to_array();
        self.text_renderer
            .render_text(pixmap, text, x, y, size, text_color);
    }

    /// Render icon with text
    /// Icon appears before text with appropriate spacing
    fn render_icon_text(
        &mut self,
        pixmap: &mut PixmapMut,
        icon_name: &str,
        text: &str,
        x: f32,
        y: f32,
        text_size: f32,
    ) {
        // Icon size is relative to text size (typically 1.2x for visibility)
        let icon_size = (text_size * 1.2) as u32;
        let icon_spacing = text_size * 0.3; // Space between icon and text

        // Load icon from cache
        match self.icon_cache.get_or_create(icon_name, icon_size) {
            Ok(icon) => {
                // Draw icon
                // Position icon vertically centered with text baseline
                let icon_y = y - (icon_size as f32 * 0.7); // Adjust for text baseline
                icon.draw(pixmap, x as i32, icon_y as i32);

                // Draw text after icon
                let text_x = x + icon_size as f32 + icon_spacing;
                self.render_text(pixmap, text, text_x, y, text_size);
            }
            Err(e) => {
                // If icon fails to load, just render text
                warn!(
                    icon = icon_name,
                    error = %e,
                    "Failed to load icon, rendering text only"
                );
                self.render_text(pixmap, text, x, y, text_size);
            }
        }
    }

    /// Draw a horizontal progress bar
    /// x_start: left edge of bar
    /// x_end: right edge of bar
    /// y: vertical position
    /// value: progress from 0.0 to 1.0
    fn draw_progress_bar(
        &self,
        pixmap: &mut PixmapMut,
        x_start: f32,
        x_end: f32,
        y: f32,
        value: f32,
    ) {
        let bar_height = 8.0;
        let total_width = x_end - x_start;
        let progress = value.clamp(0.0, 1.0);

        // Draw background track
        let mut bg_paint = Paint::default();
        let accent = self.theme.accent.to_array();
        bg_paint.set_color_rgba8(accent[0], accent[1], accent[2], 40);
        bg_paint.anti_alias = true;

        if let Some(bg_rect) = Rect::from_xywh(x_start, y, total_width, bar_height) {
            let bg_path = PathBuilder::from_rect(bg_rect);
            pixmap.fill_path(
                &bg_path,
                &bg_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        // Draw progress fill
        if progress > 0.0 {
            let mut fg_paint = Paint::default();
            fg_paint.set_color_rgba8(accent[0], accent[1], accent[2], accent[3]);
            fg_paint.anti_alias = true;

            let fill_width = total_width * progress;
            if let Some(fg_rect) = Rect::from_xywh(x_start, y, fill_width, bar_height) {
                let fg_path = PathBuilder::from_rect(fg_rect);
                pixmap.fill_path(
                    &fg_path,
                    &fg_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    /// Render dynamic widgets from registry
    pub fn render_dynamic_widgets(
        &mut self,
        canvas: &mut [u8],
        width: u32,
        height: u32,
        widgets: &[Box<dyn Widget>],
        config: &Config,
    ) {
        use crate::widget::traits::{FontSize, WidgetContent};

        // Create pixmap from canvas
        let Some(mut pixmap) = PixmapMut::from_bytes(canvas, width, height) else {
            tracing::error!("Failed to create pixmap for dynamic widgets");
            return;
        };

        // Clear background with theme color
        let bg = self.theme.background_with_opacity();
        let bg_color = bg.to_tiny_skia();
        pixmap.fill(bg_color);

        // Draw rounded rectangle background
        let corner_radius = self.theme.corner_radius;
        self.draw_rounded_rect(&mut pixmap, width, height, corner_radius, &bg);
        self.draw_rounded_border(&mut pixmap, width, height, corner_radius);

        let padding = config.padding();
        let spacing = config.panel.spacing;
        let mut y_offset = padding;

        // Render each widget
        for widget in widgets {
            let info = widget.info();
            let content = widget.content();

            // Calculate font size based on widget preference
            let font_size = match &content {
                WidgetContent::Text { size, .. } => match size {
                    FontSize::Large => 48.0,
                    FontSize::Medium => 24.0,
                    FontSize::Small => 16.0,
                    FontSize::Custom(s) => *s,
                },
                WidgetContent::MultiLine { lines } => {
                    if let Some((_, size)) = lines.first() {
                        match size {
                            FontSize::Large => 48.0,
                            FontSize::Medium => 24.0,
                            FontSize::Small => 16.0,
                            FontSize::Custom(s) => *s,
                        }
                    } else {
                        16.0
                    }
                }
                WidgetContent::IconText { size, .. } => match size {
                    FontSize::Large => 48.0,
                    FontSize::Medium => 24.0,
                    FontSize::Small => 16.0,
                    FontSize::Custom(s) => *s,
                },
                WidgetContent::Progress { .. } => 16.0,
                WidgetContent::Empty => continue,
            };

            // Render based on content type
            match content {
                WidgetContent::Text { text, .. } => {
                    // Center text horizontally
                    let text_width = self.text_renderer.measure_text(&text, font_size);
                    let x = (width as f32 - text_width) / 2.0;
                    self.render_text(&mut pixmap, &text, x, y_offset + font_size, font_size);
                    y_offset += font_size + spacing;
                }
                WidgetContent::MultiLine { lines } => {
                    for (text, size) in lines {
                        let fs = match size {
                            FontSize::Large => 48.0,
                            FontSize::Medium => 24.0,
                            FontSize::Small => 16.0,
                            FontSize::Custom(s) => s,
                        };
                        let text_width = self.text_renderer.measure_text(&text, fs);
                        let x = (width as f32 - text_width) / 2.0;
                        self.render_text(&mut pixmap, &text, x, y_offset + fs, fs);
                        y_offset += fs + spacing * 0.5;
                    }
                    y_offset += spacing * 0.5;
                }
                WidgetContent::IconText { icon, text, .. } => {
                    let x = padding;
                    self.render_icon_text(&mut pixmap, &icon, &text, x, y_offset + font_size, font_size);
                    y_offset += font_size + spacing;
                }
                WidgetContent::Progress { value, label } => {
                    // Render progress bar
                    let bar_y = y_offset + 10.0;
                    self.draw_progress_bar(&mut pixmap, padding, width as f32 - padding, bar_y, value);
                    if let Some(label_text) = label {
                        let label_width = self.text_renderer.measure_text(&label_text, 14.0);
                        let x = (width as f32 - label_width) / 2.0;
                        self.render_text(&mut pixmap, &label_text, x, bar_y + 20.0, 14.0);
                    }
                    y_offset += 30.0 + spacing;
                }
                WidgetContent::Empty => {}
            }

            tracing::trace!(widget = info.id, y_offset = y_offset, "Rendered widget");
        }

        self.first_render = false;
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
