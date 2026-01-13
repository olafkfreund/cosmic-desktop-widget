// Rendering engine using tiny-skia

use crate::config::Config;
use crate::widget::{ClockWidget, WeatherWidget};
use tiny_skia::*;

pub struct Renderer {
    font_size: f32,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            font_size: 24.0,
        }
    }

    pub fn render(
        &self,
        canvas: &mut [u8],
        width: u32,
        height: u32,
        clock: &ClockWidget,
        weather: &WeatherWidget,
        config: &Config,
    ) {
        // Create pixmap from canvas
        let mut pixmap = PixmapMut::from_bytes(canvas, width, height)
            .expect("Failed to create pixmap");

        // Clear background with semi-transparent black
        let bg_color = Color::from_rgba8(0, 0, 0, 200);
        pixmap.fill(bg_color);

        // Draw rounded rectangle background
        let rect = Rect::from_xywh(0.0, 0.0, width as f32, height as f32)
            .expect("Invalid rect");
        
        let mut paint = Paint::default();
        paint.set_color_rgba8(30, 30, 30, 230);
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
        let mut stroke = Stroke::default();
        stroke.width = 2.0;
        paint.set_color_rgba8(100, 100, 100, 255);
        
        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            None,
        );

        // Render text (simplified - in real impl use fontdue or similar)
        self.render_text(
            &mut pixmap,
            &clock.time_string(),
            20.0,
            40.0,
            32.0,
        );

        if let Some(weather_text) = weather.display_string() {
            self.render_text(
                &mut pixmap,
                &weather_text,
                20.0,
                80.0,
                20.0,
            );
        }

        // Draw decorative elements
        self.draw_decorations(&mut pixmap, width, height);
    }

    fn render_text(
        &self,
        pixmap: &mut PixmapMut,
        _text: &str,
        _x: f32,
        _y: f32,
        _size: f32,
    ) {
        // Note: tiny-skia doesn't have built-in text rendering
        // In a real implementation, use fontdue, rusttype, or ab_glyph
        // For now, just draw a placeholder rectangle
        
        let rect = Rect::from_xywh(_x, _y - _size, _size * 6.0, _size)
            .expect("Invalid text rect");
        
        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        
        let path = PathBuilder::from_rect(rect);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    fn draw_decorations(&self, pixmap: &mut PixmapMut, width: u32, height: u32) {
        // Draw a subtle accent line
        let mut paint = Paint::default();
        paint.set_color_rgba8(52, 120, 246, 255); // COSMIC blue
        
        let mut pb = PathBuilder::new();
        pb.move_to(10.0, height as f32 - 10.0);
        pb.line_to(width as f32 - 10.0, height as f32 - 10.0);
        let path = pb.finish().unwrap();
        
        let mut stroke = Stroke::default();
        stroke.width = 3.0;
        
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
