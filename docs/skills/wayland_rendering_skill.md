# Wayland Rendering Skill

## Overview

Rendering to Wayland surfaces using shared memory (wl_shm) buffers. This skill covers the complete rendering pipeline from buffer creation to surface commit.

## Buffer Management

### Shared Memory Basics

Wayland uses shared memory for efficient buffer sharing between client and compositor:

```rust
use smithay_client_toolkit::shm::slot::SlotPool;

// Create a shared memory pool
let size = (width * height * 4) as usize;  // ARGB8888 = 4 bytes/pixel
let mut pool = SlotPool::new(size * 2, &shm)?;  // 2x for double buffering
```

### Creating Buffers

```rust
let stride = width * 4;  // Bytes per row

let (buffer, canvas) = pool.create_buffer(
    width as i32,
    height as i32,
    stride as i32,
    wl_shm::Format::Argb8888,  // Format: [B, G, R, A]
)?;

// canvas is &mut [u8] - raw pixel data
```

## Pixel Formats

### ARGB8888 (Most Common)

```rust
// Format: [Blue, Green, Red, Alpha]
// Each component is 0-255

// Opaque red pixel
canvas[offset + 0] = 0;    // Blue
canvas[offset + 1] = 0;    // Green
canvas[offset + 2] = 255;  // Red
canvas[offset + 3] = 255;  // Alpha (opaque)

// Semi-transparent blue
canvas[offset + 0] = 255;  // Blue
canvas[offset + 1] = 0;    // Green
canvas[offset + 2] = 0;    // Red
canvas[offset + 3] = 128;  // Alpha (50% transparent)
```

### Working with Pixels

```rust
// Clear to transparent
canvas.fill(0);

// Fill with solid color
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = blue;
    pixel[1] = green;
    pixel[2] = red;
    pixel[3] = alpha;
}

// Draw individual pixel
fn set_pixel(canvas: &mut [u8], x: u32, y: u32, width: u32, color: [u8; 4]) {
    let offset = ((y * width + x) * 4) as usize;
    canvas[offset..offset + 4].copy_from_slice(&color);
}
```

## Using tiny-skia

### Setup

```rust
use tiny_skia::*;

// Create pixmap from existing buffer
let mut pixmap = PixmapMut::from_bytes(
    canvas, 
    width, 
    height
)?;

// Now use tiny-skia drawing functions
```

### Drawing Primitives

```rust
// Filled rectangle
let rect = Rect::from_xywh(10.0, 10.0, 100.0, 50.0)?;
let mut paint = Paint::default();
paint.set_color_rgba8(255, 0, 0, 255);  // Red

let path = PathBuilder::from_rect(rect);
pixmap.fill_path(
    &path,
    &paint,
    FillRule::Winding,
    Transform::identity(),
    None,
);

// Stroked rectangle (border)
let mut stroke = Stroke::default();
stroke.width = 2.0;
paint.set_color_rgba8(0, 0, 255, 255);  // Blue border

pixmap.stroke_path(
    &path,
    &paint,
    &stroke,
    Transform::identity(),
    None,
);
```

### Rounded Rectangles

```rust
let rect = Rect::from_xywh(10.0, 10.0, 100.0, 50.0)?;

let mut pb = PathBuilder::new();
pb.push_rounded_rect(rect, 10.0, 10.0);  // 10px corner radius
let path = pb.finish()?;

pixmap.fill_path(&path, &paint, ...);
```

### Circles

```rust
let mut pb = PathBuilder::new();
pb.push_circle(50.0, 50.0, 25.0);  // center_x, center_y, radius
let path = pb.finish()?;

pixmap.fill_path(&path, &paint, ...);
```

### Lines

```rust
let mut pb = PathBuilder::new();
pb.move_to(10.0, 10.0);
pb.line_to(100.0, 50.0);
let path = pb.finish()?;

let mut stroke = Stroke::default();
stroke.width = 3.0;

pixmap.stroke_path(&path, &paint, &stroke, ...);
```

### Gradients

```rust
// Linear gradient
let gradient = LinearGradient::new(
    Point::from_xy(0.0, 0.0),
    Point::from_xy(100.0, 0.0),
    vec![
        GradientStop::new(0.0, Color::from_rgba8(255, 0, 0, 255)),
        GradientStop::new(1.0, Color::from_rgba8(0, 0, 255, 255)),
    ],
    SpreadMode::Pad,
    Transform::identity(),
)?;

let mut paint = Paint::default();
paint.shader = gradient;

pixmap.fill_path(&path, &paint, ...);
```

## Text Rendering

### Without Font Library

tiny-skia doesn't include text rendering. You need an additional library:

```toml
[dependencies]
fontdue = "0.8"  # Font rasterization
# or
ab_glyph = "0.2"  # Alternative
```

### With fontdue

```rust
use fontdue::{Font, FontSettings};

// Load font
let font_data = include_bytes!("../assets/font.ttf");
let font = Font::from_bytes(font_data as &[u8], FontSettings::default())?;

// Rasterize text
let text = "Hello, World!";
let mut x = 10.0;
let y = 30.0;
let size = 24.0;

for ch in text.chars() {
    let (metrics, bitmap) = font.rasterize(ch, size);
    
    // Draw bitmap to pixmap
    for (i, &alpha) in bitmap.iter().enumerate() {
        let px = x + (i % metrics.width) as f32;
        let py = y + (i / metrics.width) as f32;
        
        // Blend pixel with alpha
        if px < width as f32 && py < height as f32 {
            set_pixel_with_alpha(canvas, px as u32, py as u32, width, 
                [255, 255, 255], alpha);
        }
    }
    
    x += metrics.advance_width;
}
```

## Committing to Surface

### Basic Commit

```rust
let wl_surface = layer.wl_surface();

// Mark entire surface as damaged (changed)
wl_surface.damage_buffer(
    0, 0,  // x, y offset
    width as i32, 
    height as i32
);

// Attach buffer
wl_surface.attach(Some(&buffer), 0, 0);

// Commit changes (make visible)
wl_surface.commit();
```

### Partial Damage (Performance)

```rust
// Only mark changed region as damaged
wl_surface.damage_buffer(
    changed_x, 
    changed_y, 
    changed_width, 
    changed_height
);

wl_surface.attach(Some(&buffer), 0, 0);
wl_surface.commit();
```

### Frame Callbacks

```rust
// Request notification when it's time to draw next frame
wl_surface.frame(&qh, wl_surface.clone());
wl_surface.commit();

// Handle in CompositorHandler
impl CompositorHandler for MyWidget {
    fn frame(&mut self, ..., _time: u32) {
        self.draw(qh);
    }
}
```

## Double Buffering

### Pattern 1: Two Buffers

```rust
struct BufferPair {
    buffers: [(WlBuffer, Vec<u8>); 2],
    current: usize,
}

impl BufferPair {
    fn swap(&mut self) -> (&WlBuffer, &mut [u8]) {
        self.current = (self.current + 1) % 2;
        let (buffer, data) = &mut self.buffers[self.current];
        (buffer, data.as_mut_slice())
    }
}
```

### Pattern 2: Buffer Pool

```rust
use smithay_client_toolkit::shm::slot::SlotPool;

struct BufferPool {
    pool: SlotPool,
    width: u32,
    height: u32,
}

impl BufferPool {
    fn get_buffer(&mut self) -> Result<(&WlBuffer, &mut [u8])> {
        self.pool.create_buffer(
            self.width as i32,
            self.height as i32,
            (self.width * 4) as i32,
            wl_shm::Format::Argb8888,
        )
    }
}
```

## Performance Optimization

### 1. Minimize Allocations

```rust
// ❌ BAD: Allocates every frame
fn draw(&mut self) {
    let mut canvas = vec![0u8; (self.width * self.height * 4) as usize];
    // ...
}

// ✅ GOOD: Reuse buffers
struct Renderer {
    buffer_pool: BufferPool,  // Reuses buffers
}
```

### 2. Only Redraw When Needed

```rust
struct Widget {
    needs_redraw: bool,
    last_data: Data,
}

impl Widget {
    fn update(&mut self, new_data: Data) {
        if self.last_data != new_data {
            self.last_data = new_data;
            self.needs_redraw = true;
        }
    }
    
    fn draw(&mut self) {
        if !self.needs_redraw {
            return;
        }
        
        // Render...
        self.needs_redraw = false;
    }
}
```

### 3. Use Damage Tracking

```rust
struct DamageTracker {
    dirty_regions: Vec<Rect>,
}

impl DamageTracker {
    fn mark_dirty(&mut self, rect: Rect) {
        self.dirty_regions.push(rect);
    }
    
    fn get_damage(&mut self) -> Vec<Rect> {
        std::mem::take(&mut self.dirty_regions)
    }
}
```

### 4. Throttle Frame Rate

```rust
const TARGET_FPS: u32 = 60;
const FRAME_TIME: Duration = Duration::from_millis(1000 / TARGET_FPS);

fn draw(&mut self) {
    if self.last_draw.elapsed() < FRAME_TIME {
        return;
    }
    
    // Draw...
    self.last_draw = Instant::now();
}
```

## Advanced Techniques

### Clipping

```rust
// Clip to region
let clip_rect = Rect::from_xywh(10.0, 10.0, 100.0, 100.0)?;
let clip_path = PathBuilder::from_rect(clip_rect);

pixmap.fill_path(
    &path,
    &paint,
    FillRule::Winding,
    Transform::identity(),
    Some(&clip_path),  // Clip mask
);
```

### Blending

```rust
// Alpha blending
fn blend_pixel(dst: &mut [u8], src: [u8; 4]) {
    let src_alpha = src[3] as f32 / 255.0;
    let dst_alpha = (1.0 - src_alpha);
    
    dst[0] = (src[0] as f32 * src_alpha + dst[0] as f32 * dst_alpha) as u8;
    dst[1] = (src[1] as f32 * src_alpha + dst[1] as f32 * dst_alpha) as u8;
    dst[2] = (src[2] as f32 * src_alpha + dst[2] as f32 * dst_alpha) as u8;
    dst[3] = 255;
}
```

### Image Loading

```rust
use image;

// Load PNG
let img = image::open("icon.png")?;
let img = img.to_rgba8();

// Copy to canvas
for (x, y, pixel) in img.enumerate_pixels() {
    let offset = ((y * width + x) * 4) as usize;
    canvas[offset + 0] = pixel[2];  // B
    canvas[offset + 1] = pixel[1];  // G
    canvas[offset + 2] = pixel[0];  // R
    canvas[offset + 3] = pixel[3];  // A
}
```

## Common Patterns

### Background Fill

```rust
// Transparent
canvas.fill(0);

// Solid color
for pixel in canvas.chunks_exact_mut(4) {
    pixel.copy_from_slice(&[0, 0, 0, 255]);  // Opaque black
}

// Semi-transparent
for pixel in canvas.chunks_exact_mut(4) {
    pixel.copy_from_slice(&[30, 30, 30, 230]);  // Dark gray, 90% opacity
}
```

### Rounded Corner Widget

```rust
fn draw_widget_background(pixmap: &mut PixmapMut, width: u32, height: u32) {
    let rect = Rect::from_xywh(0.0, 0.0, width as f32, height as f32)?;
    
    // Rounded rectangle
    let mut pb = PathBuilder::new();
    pb.push_rounded_rect(rect, 10.0, 10.0);
    let path = pb.finish()?;
    
    // Background
    let mut paint = Paint::default();
    paint.set_color_rgba8(30, 30, 30, 230);
    paint.anti_alias = true;
    
    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    
    // Border
    let mut stroke = Stroke::default();
    stroke.width = 2.0;
    paint.set_color_rgba8(100, 100, 100, 255);
    
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}
```

### Progress Bar

```rust
fn draw_progress_bar(
    pixmap: &mut PixmapMut, 
    x: f32, 
    y: f32, 
    width: f32, 
    height: f32,
    progress: f32  // 0.0 to 1.0
) {
    // Background
    let bg_rect = Rect::from_xywh(x, y, width, height)?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(50, 50, 50, 255);
    
    let path = PathBuilder::from_rect(bg_rect);
    pixmap.fill_path(&path, &paint, ...);
    
    // Progress fill
    let progress_width = width * progress;
    let fg_rect = Rect::from_xywh(x, y, progress_width, height)?;
    paint.set_color_rgba8(52, 120, 246, 255);  // COSMIC blue
    
    let path = PathBuilder::from_rect(fg_rect);
    pixmap.fill_path(&path, &paint, ...);
}
```

## Debugging

### Visualize Buffer

```rust
// Save buffer to PNG for inspection
use image::{ImageBuffer, Rgba};

let img = ImageBuffer::<Rgba<u8>, _>::from_raw(
    width,
    height,
    canvas.to_vec(),
)?;

img.save("debug_output.png")?;
```

### Check Pixel Values

```rust
fn debug_pixel(canvas: &[u8], x: u32, y: u32, width: u32) {
    let offset = ((y * width + x) * 4) as usize;
    println!("Pixel at ({}, {}): B={}, G={}, R={}, A={}", 
        x, y,
        canvas[offset + 0],
        canvas[offset + 1],
        canvas[offset + 2],
        canvas[offset + 3]
    );
}
```

### Performance Profiling

```rust
use std::time::Instant;

fn draw(&mut self) {
    let start = Instant::now();
    
    // Rendering code...
    
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 16 {  // > 16ms = < 60 FPS
        tracing::warn!("Slow render: {:?}", elapsed);
    }
}
```

## Best Practices

1. **Reuse buffers** - Don't allocate every frame
2. **Use damage tracking** - Only redraw changed regions
3. **Throttle updates** - Don't render faster than display refresh
4. **Profile rendering** - Measure actual performance
5. **Pre-render static content** - Cache unchanging elements
6. **Use appropriate pixel format** - ARGB8888 is standard

## Resources

- [tiny-skia docs](https://docs.rs/tiny-skia/)
- [fontdue docs](https://docs.rs/fontdue/)
- [Wayland shm protocol](https://wayland.app/protocols/wayland#wl_shm)
- [Smithay Client Toolkit](https://smithay.github.io/client-toolkit/)

---

**Last Updated**: 2025-01-13
