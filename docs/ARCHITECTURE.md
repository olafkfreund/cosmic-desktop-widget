# Architecture Guide

This document describes the internal architecture of COSMIC Desktop Widget, providing insight into the design decisions, component interactions, and implementation details.

## Table of Contents

1. [Overview](#overview)
2. [Layer Shell Architecture](#layer-shell-architecture)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Module Reference](#module-reference)
6. [Widget System](#widget-system)
7. [Rendering Pipeline](#rendering-pipeline)
8. [Update Scheduling](#update-scheduling)
9. [Performance Architecture](#performance-architecture)
10. [Error Handling](#error-handling)
11. [Extending the Widget](#extending-the-widget)

## Overview

COSMIC Desktop Widget is a Wayland-native desktop widget application that uses the Layer Shell protocol to render widgets directly on the desktop background. The architecture prioritizes:

- **Native Wayland Integration** - Direct protocol implementation without compatibility layers
- **Performance** - Efficient rendering with caching and smart updates
- **Modularity** - Clean separation between widgets, rendering, and Wayland handling
- **Extensibility** - Easy addition of new widget types

### High-Level Architecture

```
+----------------------------------------------------------+
|                     User Space                           |
+----------------------------------------------------------+
|                                                          |
|  +--------------------+    +-------------------------+   |
|  | Configuration      |    | Widget System           |   |
|  | (config.toml)      |--->| (Clock, Weather, ...)   |   |
|  +--------------------+    +-------------------------+   |
|                                       |                  |
|  +--------------------+               v                  |
|  | Theme System       |    +-------------------------+   |
|  | (colors, styles)   |--->| Renderer (tiny-skia)    |   |
|  +--------------------+    +-------------------------+   |
|                                       |                  |
|  +--------------------+               v                  |
|  | Layout Manager     |    +-------------------------+   |
|  | (positioning)      |--->| Buffer Pool (wl_shm)    |   |
|  +--------------------+    +-------------------------+   |
|                                       |                  |
+----------------------------------------------------------+
|                                       v                  |
|  +----------------------------------------------------+  |
|  |              Wayland Protocol Layer                |  |
|  |  +-------------+  +------------+  +-------------+  |  |
|  |  | Layer Shell |  | Compositor |  | Shared Mem  |  |  |
|  |  | (zwlr)      |  | (wl_comp)  |  | (wl_shm)    |  |  |
|  |  +-------------+  +------------+  +-------------+  |  |
|  +----------------------------------------------------+  |
|                                                          |
+----------------------------------------------------------+
|                   Compositor (COSMIC/Sway/etc)           |
+----------------------------------------------------------+
```

## Layer Shell Architecture

### What is Layer Shell?

The Layer Shell protocol (`zwlr_layer_shell_v1`) is a wlroots protocol extension that allows clients to create surfaces at specific layers in the compositor's rendering stack:

```
+-----------------------------------+
| Overlay Layer                     |  <- Lock screens, critical notifications
+-----------------------------------+
| Top Layer                         |  <- Panels, docks, OSD
+-----------------------------------+
| Regular Windows (XDG Shell)       |  <- Normal applications
+-----------------------------------+
| Bottom Layer  <-- WE ARE HERE     |  <- Desktop widgets
+-----------------------------------+
| Background Layer                  |  <- Wallpaper
+-----------------------------------+
```

### Surface Configuration

Our widget creates a Layer Shell surface with these properties:

```rust
// Layer: Bottom (below windows, above wallpaper)
layer.set_layer(Layer::Bottom);

// Anchor: Determines which screen edge(s) to attach to
layer.set_anchor(Anchor::TOP | Anchor::RIGHT);

// Size: Widget dimensions
layer.set_size(400, 150);

// Margins: Offset from anchored edges
layer.set_margin(top, right, bottom, left);

// Keyboard: No keyboard focus (non-interactive)
layer.set_keyboard_interactivity(KeyboardInteractivity::None);

// Exclusive zone: -1 means don't reserve space
layer.set_exclusive_zone(-1);
```

### Protocol Flow

```
Client                              Compositor
  |                                     |
  |-- get_layer_surface() ------------->|
  |                                     |
  |<-- configure(width, height) --------|
  |                                     |
  |-- set_anchor(TOP|RIGHT) ----------->|
  |-- set_size(400, 150) -------------->|
  |-- set_margin(20, 20, 0, 0) -------->|
  |-- commit() ------------------------>|
  |                                     |
  |<-- configure(400, 150) -------------|
  |                                     |
  |-- ack_configure() ----------------->|
  |-- attach(buffer) ------------------>|
  |-- damage(region) ------------------>|
  |-- commit() ------------------------>|
  |                                     |
  |<-- frame() -------------------------|  (ready for next frame)
  |                                     |
```

## Component Architecture

### Core Components

```
src/
+-- main.rs              # Application entry point, event loop
+-- lib.rs               # Library exports and re-exports
+-- config/
|   +-- mod.rs           # Config struct, loading, validation
+-- theme/
|   +-- mod.rs           # Color, Theme structs, built-in themes
+-- widget/
|   +-- mod.rs           # ClockWidget, WeatherWidget
+-- render/
|   +-- mod.rs           # Renderer, tiny-skia integration
+-- layout/
|   +-- mod.rs           # LayoutManager, WidgetPosition
+-- text/
|   +-- mod.rs           # Text module exports
|   +-- font.rs          # Font loading from system
|   +-- renderer.rs      # Text rendering with fontdue
|   +-- glyph_cache.rs   # LRU glyph cache
+-- wayland/
|   +-- mod.rs           # BufferPool, shared memory management
+-- update/
|   +-- mod.rs           # UpdateScheduler, UpdateFlags
+-- metrics/
|   +-- mod.rs           # RenderMetrics, CacheMetrics, Timer
+-- weather/
|   +-- mod.rs           # Weather API types
+-- error.rs             # Error types (WeatherError, ConfigError, etc.)
```

### Dependency Graph

```
main.rs
    |
    +-- DesktopWidget (application state)
    |       |
    |       +-- Config
    |       +-- Renderer
    |       |       +-- TextRenderer
    |       |       |       +-- GlyphCache
    |       |       |       +-- Font
    |       |       +-- Theme
    |       |       +-- LayoutManager
    |       +-- ClockWidget
    |       +-- WeatherWidget
    |       +-- UpdateScheduler
    |       +-- WidgetMetrics
    |       +-- BufferPool
    |
    +-- Wayland Protocol Handlers
            +-- CompositorHandler
            +-- LayerShellHandler
            +-- ShmHandler
            +-- OutputHandler
```

## Data Flow

### Initialization Flow

```
1. main()
   |
   +-> Initialize tracing/logging
   |
   +-> Config::load()
   |       +-> Read ~/.config/cosmic-desktop-widget/config.toml
   |       +-> Validate configuration
   |       +-> Return Config or defaults
   |
   +-> Connection::connect_to_env()
   |       +-> Connect to Wayland display
   |
   +-> registry_queue_init()
   |       +-> Get global registry
   |       +-> Bind required protocols
   |
   +-> DesktopWidget::new()
   |       +-> Initialize widgets from config
   |       +-> Create Renderer with theme
   |       +-> Setup UpdateScheduler
   |
   +-> create_layer_surface()
   |       +-> Create wl_surface
   |       +-> Create layer_surface (Bottom layer)
   |       +-> Configure anchor, size, margins
   |
   +-> EventLoop::dispatch()
           +-> Enter main event loop
```

### Render Flow

```
1. Event Loop Tick
   |
   +-> UpdateScheduler::check_updates()
   |       +-> Check clock interval (1 second)
   |       +-> Check weather interval (config)
   |       +-> Return UpdateFlags
   |
   +-> If needs_redraw():
   |   |
   |   +-> Update widgets that need updating
   |   |       +-> ClockWidget::update()
   |   |       +-> WeatherWidget::update()
   |   |
   |   +-> BufferPool::get_buffer()
   |   |       +-> Get or create shared memory buffer
   |   |       +-> Return (WlBuffer, &mut [u8])
   |   |
   |   +-> Renderer::render()
   |   |       +-> Create PixmapMut from buffer
   |   |       +-> Fill background (theme color)
   |   |       +-> Draw border
   |   |       +-> Calculate layout positions
   |   |       +-> Render clock text
   |   |       +-> Render weather text
   |   |       +-> Draw decorations
   |   |
   |   +-> Attach buffer to surface
   |   +-> Mark damage region
   |   +-> Commit surface
   |
   +-> Continue event loop
```

### Weather Update Flow

```
1. UpdateScheduler signals weather update needed
   |
   +-> WeatherWidget::fetch_weather() (async)
   |       |
   |       +-> Validate API key exists
   |       |
   |       +-> reqwest::get(openweathermap_url)
   |       |       +-> HTTP GET to API
   |       |       +-> Parse JSON response
   |       |
   |       +-> Extract weather data
   |       |       +-> temperature
   |       |       +-> condition
   |       |       +-> humidity
   |       |       +-> wind_speed
   |       |
   |       +-> WeatherWidget::set_data() or set_error()
   |
   +-> Next render cycle displays new data
```

## Module Reference

### config (Configuration Management)

**Purpose:** Load, validate, and manage widget configuration.

**Key Types:**
- `Config` - Main configuration struct
- `Margin` - Screen edge margins

**Key Functions:**
- `Config::load()` - Load from file or create defaults
- `Config::save()` - Write config to file
- `Config::validate()` - Validate configuration values
- `Config::get_theme()` - Get Theme from config

**Design Decisions:**
- Resilient to errors (uses defaults on failure)
- Validates early to catch configuration issues
- TOML format for human-readable config

### theme (Theming System)

**Purpose:** Define visual appearance with colors and styles.

**Key Types:**
- `Color` - RGBA color with conversion methods
- `Theme` - Complete theme definition

**Key Functions:**
- `Theme::cosmic_dark()` - Default dark theme
- `Theme::light()` - Light theme variant
- `Theme::transparent_dark()` - Transparent variant
- `Theme::from_name()` - Load theme by name
- `Color::to_tiny_skia()` - Convert to rendering color

**Design Decisions:**
- Themes are self-contained (all colors in one struct)
- Built-in themes match COSMIC Desktop aesthetics
- Custom themes supported via config

### widget (Widget Implementations)

**Purpose:** Implement individual widget types (Clock, Weather).

**Key Types:**
- `ClockWidget` - Time display widget
- `WeatherWidget` - Weather data widget
- `WeatherData` - Parsed weather information

**Key Functions:**
- `ClockWidget::update()` - Update time string
- `ClockWidget::time_string()` - Get formatted time
- `WeatherWidget::set_data()` - Set weather from API
- `WeatherWidget::set_error()` - Set error state
- `WeatherWidget::display_string()` - Get display text

**Design Decisions:**
- Widgets are data-focused (no rendering logic)
- Weather uses set_data/set_error pattern for async updates
- Clock handles format conversion internally

### render (Rendering Engine)

**Purpose:** Render widgets to pixel buffers using tiny-skia.

**Key Types:**
- `Renderer` - Main rendering coordinator

**Key Functions:**
- `Renderer::render()` - Main render function
- `Renderer::render_text()` - Render text with theme color
- `Renderer::draw_decorations()` - Draw accent elements

**Design Decisions:**
- Uses tiny-skia for pure Rust 2D rendering
- Renders to ARGB8888 pixel buffer
- Theme-aware (uses theme colors)

### layout (Layout System)

**Purpose:** Calculate widget positions within container.

**Key Types:**
- `LayoutManager` - Layout calculation
- `WidgetPosition` - x, y, width, height
- `LayoutDirection` - Vertical or Horizontal

**Key Functions:**
- `LayoutManager::calculate_positions()` - Generic layout
- `LayoutManager::clock_position()` - Clock-specific position
- `LayoutManager::weather_position()` - Weather-specific position

**Design Decisions:**
- Configurable padding and spacing
- Supports both vertical and horizontal layouts
- Context-aware (clock position depends on weather visibility)

### text (Text Rendering)

**Purpose:** High-quality text rendering with caching.

**Key Types:**
- `TextRenderer` - Font loading and text rendering
- `GlyphCache` - LRU cache for rasterized glyphs
- `CachedGlyph` - Cached glyph bitmap

**Key Functions:**
- `TextRenderer::render_text()` - Render string at position
- `GlyphCache::get_or_rasterize()` - Get cached or create glyph

**Design Decisions:**
- Uses fontdue for font rasterization
- LRU cache prevents repeated rasterization
- Loads fonts from system via fontconfig patterns

### wayland (Wayland Integration)

**Purpose:** Manage Wayland buffers and shared memory.

**Key Types:**
- `BufferPool` - Manages shared memory buffers

**Key Functions:**
- `BufferPool::new()` - Create buffer pool
- `BufferPool::get_buffer()` - Get buffer for rendering

**Design Decisions:**
- Double-buffering for smooth updates
- Uses wl_shm for shared memory
- Buffer size matches widget dimensions

### update (Update Scheduling)

**Purpose:** Coordinate widget update timing.

**Key Types:**
- `UpdateScheduler` - Tracks update intervals
- `UpdateFlags` - Which components need updating

**Key Functions:**
- `UpdateScheduler::check_updates()` - Check what needs updating
- `UpdateScheduler::force_update_all()` - Force immediate update
- `UpdateFlags::needs_redraw()` - Check if any flag set

**Design Decisions:**
- Separate intervals for clock (1s) and weather (configurable)
- Only triggers render when something changed
- Reduces CPU usage significantly

### metrics (Performance Tracking)

**Purpose:** Track and report performance metrics.

**Key Types:**
- `RenderMetrics` - Render timing stats
- `CacheMetrics` - Cache hit/miss tracking
- `WidgetMetrics` - Aggregated metrics
- `Timer` - Simple duration timer

**Key Functions:**
- `RenderMetrics::record_render()` - Record render duration
- `CacheMetrics::record_hit/miss()` - Track cache efficiency
- `WidgetMetrics::maybe_log_summary()` - Periodic logging

**Design Decisions:**
- Minimal overhead when not logging
- Periodic summaries (every 60 seconds)
- Tracks frame budget compliance

## Widget System

### Current Widget Types

**ClockWidget:**
- Displays current time
- Configurable format (12h/24h)
- Optional seconds and date
- Updates every second

**WeatherWidget:**
- Displays current weather conditions
- OpenWeatherMap API integration
- Temperature unit conversion
- Stale data and error indicators

### Widget Interface Pattern

While not using a formal trait, widgets follow this pattern:

```rust
pub struct SomeWidget {
    // Internal state
    data: SomeData,
    last_update: Instant,
}

impl SomeWidget {
    // Constructor
    pub fn new(config: &str) -> Self { ... }

    // Update internal state
    pub fn update(&mut self) { ... }

    // Get display string for rendering
    pub fn display_string(&self) -> String { ... }
}
```

### Adding a New Widget

1. **Create widget struct:**
```rust
// src/widget/mod.rs
pub struct SystemMonitorWidget {
    cpu_percent: f32,
    memory_percent: f32,
    last_update: Instant,
}

impl SystemMonitorWidget {
    pub fn new() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_percent: 0.0,
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        // Read from /proc or use sysinfo crate
        // Update cpu_percent, memory_percent
        self.last_update = Instant::now();
    }

    pub fn display_string(&self) -> String {
        format!("CPU: {:.0}% | RAM: {:.0}%",
            self.cpu_percent, self.memory_percent)
    }
}
```

2. **Add configuration:**
```rust
// src/config/mod.rs
pub struct Config {
    // ...existing fields...
    pub show_system_monitor: bool,
}
```

3. **Add to DesktopWidget:**
```rust
// src/main.rs
struct DesktopWidget {
    // ...existing fields...
    system_monitor: Option<SystemMonitorWidget>,
}
```

4. **Update rendering:**
```rust
// src/render/mod.rs
pub fn render(&mut self, ..., system_monitor: Option<&SystemMonitorWidget>) {
    // ...existing rendering...
    if let Some(monitor) = system_monitor {
        let text = monitor.display_string();
        self.render_text(&mut pixmap, &text, x, y, size);
    }
}
```

5. **Update scheduler:**
```rust
// src/update/mod.rs
pub struct UpdateFlags {
    // ...existing fields...
    pub system_monitor: bool,
}
```

## Rendering Pipeline

### Buffer Format

The widget uses ARGB8888 format (pre-multiplied alpha):

```
Byte order: [Blue, Green, Red, Alpha]
Size: width * height * 4 bytes
Stride: width * 4 bytes
```

### Rendering Steps

1. **Clear Background:**
   ```rust
   pixmap.fill(background_color);
   ```

2. **Draw Background Shape:**
   ```rust
   let rect = Rect::from_xywh(0, 0, width, height);
   pixmap.fill_path(&rect_path, &paint, ...);
   ```

3. **Draw Border:**
   ```rust
   pixmap.stroke_path(&rect_path, &border_paint, &stroke, ...);
   ```

4. **Render Text:**
   ```rust
   for glyph in text.chars() {
       let cached = cache.get_or_rasterize(glyph, size);
       // Blend glyph bitmap into pixmap
   }
   ```

5. **Draw Decorations:**
   ```rust
   // Accent line at bottom
   pixmap.stroke_path(&line_path, &accent_paint, ...);
   ```

### Text Rendering Pipeline

```
1. Input: "14:35:22"
   |
2. For each character:
   |
   +-> Check GlyphCache
   |       |
   |       +-> Cache HIT: Return cached bitmap
   |       |
   |       +-> Cache MISS:
   |               +-> fontdue::Font::rasterize()
   |               +-> Store in cache
   |               +-> Return bitmap
   |
3. Blend glyph bitmap into pixmap at (x, y)
   |
4. Advance x by glyph width
```

## Update Scheduling

### Timing Strategy

```
Clock: Update every 1 second
       +-- Lightweight (string formatting)
       +-- Must be responsive (user expectation)

Weather: Update every N seconds (configurable, default 600)
         +-- Expensive (network request)
         +-- Data changes slowly
         +-- API rate limits

Render: Only when data changes
        +-- Check UpdateFlags before rendering
        +-- Skip render if nothing changed
```

### UpdateScheduler Implementation

```rust
pub struct UpdateScheduler {
    clock_interval: Duration,
    weather_interval: Duration,
    last_clock_update: Instant,
    last_weather_update: Instant,
}

impl UpdateScheduler {
    pub fn check_updates(&mut self) -> UpdateFlags {
        let mut flags = UpdateFlags::new();

        if self.last_clock_update.elapsed() >= self.clock_interval {
            flags.clock = true;
            self.last_clock_update = Instant::now();
        }

        if self.last_weather_update.elapsed() >= self.weather_interval {
            flags.weather = true;
            self.last_weather_update = Instant::now();
        }

        flags
    }
}
```

## Performance Architecture

### Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Idle CPU | < 0.1% | Desktop widget should be invisible |
| Active CPU | < 1% | Brief spikes during updates acceptable |
| Memory | < 50 MB | Including buffers and caches |
| Render time | < 16ms | 60fps frame budget |

### Optimization Strategies

**1. Glyph Caching:**
```
First render of "14:35:22":
  - 8 cache misses (new glyphs)
  - 8 rasterizations

Subsequent renders:
  - 8 cache hits (most of the time)
  - 0 rasterizations
```

**2. Smart Updates:**
```
Without smart updates:
  - 60 renders per second
  - Always redraw everything

With smart updates:
  - 1 render per second (clock)
  - 1 render per 600 seconds (weather)
  - Skip render if no changes
```

**3. Buffer Reuse:**
```
Without reuse:
  - Allocate new buffer each frame
  - High allocation overhead

With BufferPool:
  - Reuse existing buffers
  - Minimal allocations after warmup
```

### Metrics Collection

```rust
// Record render time
let timer = Timer::start();
self.renderer.render(...);
let duration = timer.stop();
self.metrics.render.record_render(duration);

// Log if over budget
if duration.as_millis() > 16 {
    tracing::warn!("Render exceeded frame budget");
}

// Periodic summary (every 60 seconds)
self.metrics.maybe_log_summary();
```

## Error Handling

### Error Types

```rust
// src/error.rs
pub enum WidgetError {
    Config(ConfigError),
    Weather(WeatherError),
    Render(RenderError),
    Wayland(WaylandError),
}

pub enum WeatherError {
    NoApiKey,
    NetworkError(reqwest::Error),
    CityNotFound(String),
    ParseError(String),
}

pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
    ValidationError(String),
}
```

### Error Handling Strategy

**Configuration Errors:**
- Log warning
- Use default values
- Continue running

**Weather Errors:**
- Log warning
- Keep displaying old data (if available)
- Show error indicator in display
- Retry on next interval

**Render Errors:**
- Log error
- Skip frame
- Try again next frame

**Wayland Errors:**
- Log error
- Exit gracefully (most are fatal)

### Graceful Degradation

```
API key missing:
  +-- Weather widget shows "No API key"
  +-- Clock continues working

Network error:
  +-- Weather shows last known data + warning
  +-- Clock continues working

Font loading fails:
  +-- Try fallback fonts
  +-- Log error if all fail
```

## Extending the Widget

### Adding a New Feature

1. **Plan the feature:**
   - What data does it need?
   - How often does it update?
   - Where does data come from?

2. **Implement data source:**
   - Create widget struct
   - Implement update logic
   - Handle errors gracefully

3. **Add configuration:**
   - Add config options
   - Provide sensible defaults
   - Add validation

4. **Integrate with renderer:**
   - Calculate layout position
   - Render content
   - Apply theme colors

5. **Add to scheduler:**
   - Determine update interval
   - Add flag to UpdateFlags
   - Update scheduler logic

6. **Test:**
   - Unit tests for logic
   - Integration tests for workflow
   - Manual testing on compositor

### Best Practices

**Code Style:**
- Use `thiserror` for error types
- Use `tracing` for logging
- Avoid `unwrap()` in production code
- Document public APIs

**Performance:**
- Cache expensive operations
- Only update when needed
- Reuse allocations
- Profile before optimizing

**Wayland:**
- Always check `configured` before drawing
- Handle surface closure gracefully
- Reuse buffers efficiently
- Clean up on drop
