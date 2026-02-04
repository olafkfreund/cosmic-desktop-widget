// Rendering engine using tiny-skia

use crate::config::Config;
use crate::layout::LayoutManager;
use crate::text::TextRenderer;
use crate::theme::Theme;
use crate::widget::{ClockWidget, WeatherWidget};
use tiny_skia::*;
use tracing::{instrument, trace};

pub struct Renderer {
    #[allow(dead_code)]
    font_size: f32,
    text_renderer: TextRenderer,
    theme: Theme,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            font_size: 24.0,
            text_renderer: TextRenderer::new(),
            theme: Theme::default(),
        }
    }

    pub fn with_theme(theme: Theme) -> Self {
        Self {
            font_size: 24.0,
            text_renderer: TextRenderer::new(),
            theme,
        }
    }

    #[instrument(skip(self, canvas, clock, weather, _config), fields(width = %width, height = %height))]
    pub fn render(
        &mut self,
        canvas: &mut [u8],
        width: u32,
        height: u32,
        clock: Option<&ClockWidget>,
        weather: Option<&WeatherWidget>,
        _config: &Config,
    ) {
        trace!("Starting render");

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

        // Draw rounded rectangle background
        let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) else {
            tracing::error!(
                width = width,
                height = height,
                "Failed to create rectangle - invalid dimensions"
            );
            return;
        };

        let mut paint = Paint::default();
        let bg_rgba = bg.to_array();
        paint.set_color_rgba8(bg_rgba[0], bg_rgba[1], bg_rgba[2], bg_rgba[3]);
        paint.anti_alias = true;

        let path = PathBuilder::from_rect(rect);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        // Draw border
        let stroke = Stroke {
            width: self.theme.border_width,
            ..Default::default()
        };
        let border = self.theme.border.to_array();
        paint.set_color_rgba8(border[0], border[1], border[2], border[3]);

        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            None,
        );

        // Create layout manager
        let layout = LayoutManager::new(width, height);

        // Render clock if enabled
        if let Some(clock_widget) = clock {
            let pos = layout.clock_position(weather.is_some());
            self.render_text(
                &mut pixmap,
                &clock_widget.time_string(),
                pos.x,
                pos.y + 32.0, // Add font size for baseline positioning
                32.0,
            );
        }

        // Render weather if enabled
        if let Some(weather_widget) = weather {
            if let Some(weather_text) = weather_widget.display_string() {
                let pos = layout.weather_position(clock.is_some());
                self.render_text(
                    &mut pixmap,
                    &weather_text,
                    pos.x,
                    pos.y + 20.0, // Add font size for baseline positioning
                    20.0,
                );
            }
        }

        // Draw decorative elements
        self.draw_decorations(&mut pixmap, width, height);

        trace!("Render complete");
    }

    fn render_text(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
    ) {
        // Render text using fontdue with theme color
        let text_color = self.theme.text_primary.to_array();
        self.text_renderer.render_text(
            pixmap,
            text,
            x,
            y,
            size,
            text_color,
        );
    }

    fn draw_decorations(&self, pixmap: &mut PixmapMut, width: u32, height: u32) {
        // Draw a subtle accent line with theme color
        let mut paint = Paint::default();
        let accent = self.theme.accent.to_array();
        paint.set_color_rgba8(accent[0], accent[1], accent[2], accent[3]);

        let mut pb = PathBuilder::new();
        pb.move_to(10.0, height as f32 - 10.0);
        pb.line_to(width as f32 - 10.0, height as f32 - 10.0);
        let Some(path) = pb.finish() else {
            tracing::warn!("Failed to create decoration path, skipping decorations");
            return;
        };

        let stroke = Stroke {
            width: 3.0,
            ..Default::default()
        };

        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            None,
        );
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
