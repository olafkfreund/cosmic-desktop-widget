# CLAUDE.md - AI Assistant Context for COSMIC Desktop Widget

## Project Identity

**Name:** COSMIC Desktop Widget  
**Type:** Wayland Layer Shell Desktop Widget System  
**Language:** Rust  
**Platform:** Linux (Wayland compositors)  
**Status:** Active Development  
**License:** GPL-3.0

## Project Purpose

Build true desktop widgets for COSMIC Desktop Environment using the Wayland Layer Shell protocol (`zwlr_layer_shell_v1`). Widgets live on the desktop background (below windows, above wallpaper) - not in panels or as floating windows.

**Think:** KDE Plasma widgets, Windows desktop gadgets, but for COSMIC Desktop using modern Wayland protocols.

## Technology Stack

### Core Technologies
- **Wayland Layer Shell** - zwlr_layer_shell_v1 protocol for desktop surfaces
- **Smithay Client Toolkit** - Wayland client library for Rust
- **tiny-skia** - 2D graphics rendering engine
- **calloop** - Event loop for Wayland events

### Dependencies
```toml
smithay-client-toolkit = "0.18"  # Wayland client
wayland-client = "0.31"          # Wayland protocol
wayland-protocols-wlr = "0.2"    # wlroots protocols (Layer Shell)
calloop = "0.12"                 # Event loop
tiny-skia = "0.11"               # Rendering
reqwest = "0.12"                 # HTTP client (weather API)
tokio = "1"                      # Async runtime
chrono = "0.4"                   # Time handling
toml = "0.8"                     # Configuration
```

## Project Structure

```
cosmic-desktop-widget/
├── src/
│   ├── main.rs              # Entry point, Layer Shell setup
│   ├── wayland/
│   │   └── mod.rs           # Buffer pool, shared memory
│   ├── render/
│   │   └── mod.rs           # Rendering with tiny-skia
│   ├── widget/
│   │   └── mod.rs           # Widget implementations
│   └── config/
│       └── mod.rs           # Configuration management
├── docs/
│   └── skills/              # AI assistant skills
├── Cargo.toml               # Dependencies
├── flake.nix                # NixOS build
├── justfile                 # Build automation
└── README.md                # User documentation
```

## Architecture Overview

### Layer Shell Widget Stack

```
┌─────────────────────────────────┐
│  Application Windows            │  <- XDG Shell (normal windows)
├─────────────────────────────────┤
│  OUR WIDGET (Bottom Layer)      │  <- Layer Shell (our surface)
├─────────────────────────────────┤
│  Wallpaper                      │  <- Background layer
└─────────────────────────────────┘
```

### Code Architecture

```
main.rs
  ├─→ Wayland Connection
  │     └─→ Layer Shell Protocol
  │           └─→ Surface Creation (Bottom layer)
  │
  ├─→ Event Loop (calloop)
  │     ├─→ Wayland events (configure, frame)
  │     ├─→ Timer events (widget updates)
  │     └─→ Signal handling (graceful shutdown)
  │
  ├─→ Buffer Management (wayland/mod.rs)
  │     └─→ Shared memory buffers (wl_shm)
  │           └─→ Double buffering
  │
  ├─→ Widget System (widget/mod.rs)
  │     ├─→ ClockWidget (time display)
  │     └─→ WeatherWidget (API integration)
  │
  └─→ Rendering Pipeline (render/mod.rs)
        └─→ tiny-skia canvas
              └─→ Pixel buffer output
```

## Coding Standards

### Rust Style

```rust
// ✅ GOOD: Clear naming, proper error handling
pub fn create_layer_surface(
    &mut self, 
    qh: &QueueHandle<Self>
) -> Result<(), Error> {
    let surface = self.compositor_state
        .create_surface(qh);
    
    let layer = self.layer_shell
        .create_layer_surface(
            qh,
            surface,
            Layer::Bottom,
            Some("widget"),
            None,
        );
    
    layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
    layer.commit();
    
    self.layer = Some(layer);
    Ok(())
}

// ❌ BAD: Unwrap in production, unclear naming
pub fn make_surface(&mut self, q: &QueueHandle<Self>) {
    let s = self.cs.create_surface(q);
    let l = self.ls.create_layer_surface(
        q, s, Layer::Bottom, Some("w"), None
    ).unwrap(); // DON'T UNWRAP!
    self.layer = Some(l);
}
```

### Error Handling

```rust
// ✅ Use Result for fallible operations
pub fn fetch_weather(&mut self) -> Result<WeatherData, WeatherError> {
    let response = reqwest::blocking::get(&self.api_url)?;
    let data = response.json()?;
    Ok(data)
}

// ✅ Proper error propagation
pub fn update(&mut self) -> Result<(), Error> {
    self.clock_widget.update()?;
    self.weather_widget.update()?;
    self.render()?;
    Ok(())
}

// ❌ Don't use unwrap() or expect() in production
let data = response.json().unwrap(); // BAD!
```

### Async Patterns

```rust
// ✅ Non-blocking async for I/O
async fn fetch_weather(&self) -> Result<WeatherData> {
    let response = reqwest::get(&self.api_url).await?;
    let data = response.json().await?;
    Ok(data)
}

// ❌ Don't block the event loop
fn update(&mut self) {
    std::thread::sleep(Duration::from_secs(5)); // BAD!
    let data = self.fetch_weather_blocking(); // BAD!
}
```

### Resource Management

```rust
// ✅ Implement Drop for cleanup
impl Drop for DesktopWidget {
    fn drop(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.destroy();
        }
        tracing::info!("Widget cleaned up");
    }
}

// ✅ Use RAII patterns
pub struct BufferPool {
    pool: SlotPool,
    // Automatically cleaned up on drop
}
```

## Layer Shell Patterns

### Surface Creation

```rust
// Standard Layer Shell surface creation
let layer = layer_shell.create_layer_surface(
    qh,
    wl_surface,
    Layer::Bottom,           // Z-order layer
    Some("widget-id"),       // Unique namespace
    None,                    // Output (None = all)
);

// Configuration
layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
layer.set_size(width, height);
layer.set_margin(top, right, bottom, left);
layer.set_keyboard_interactivity(KeyboardInteractivity::None);
layer.set_exclusive_zone(-1);  // Don't reserve space
layer.commit();
```

### Event Handling

```rust
impl LayerShellHandler for DesktopWidget {
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Update size if compositor changed it
        if configure.new_size.0 > 0 {
            self.width = configure.new_size.0;
            self.height = configure.new_size.1;
        }
        
        self.configured = true;
        self.draw(qh);
    }

    fn closed(&mut self, ...) {
        tracing::info!("Layer surface closed");
        // Cleanup and exit
    }
}
```

### Buffer Management

```rust
// Create buffer pool
let pool = SlotPool::new(size, &shm_state)?;

// Get buffer
let (buffer, canvas) = pool.create_buffer(
    width as i32,
    height as i32,
    stride as i32,
    wl_shm::Format::Argb8888,
)?;

// Draw to canvas (ARGB8888 format)
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = blue;
    pixel[1] = green;
    pixel[2] = red;
    pixel[3] = alpha;
}

// Attach and commit
surface.attach(Some(&buffer), 0, 0);
surface.damage_buffer(0, 0, width as i32, height as i32);
surface.commit();
```

## Widget Implementation Pattern

### Widget Trait (Future)

```rust
pub trait Widget {
    fn update(&mut self) -> Result<()>;
    fn render(&self, canvas: &mut Canvas) -> Result<()>;
    fn size(&self) -> (u32, u32);
}

// Example implementation
pub struct ClockWidget {
    time: String,
    last_update: Instant,
}

impl Widget for ClockWidget {
    fn update(&mut self) -> Result<()> {
        if self.last_update.elapsed() >= Duration::from_secs(1) {
            self.time = Local::now().format("%H:%M:%S").to_string();
            self.last_update = Instant::now();
        }
        Ok(())
    }
    
    fn render(&self, canvas: &mut Canvas) -> Result<()> {
        // Render time string
        Ok(())
    }
}
```

## Configuration System

```rust
// TOML-based configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub position: String,
    pub margin: Margin,
    pub weather_city: String,
    pub weather_api_key: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }
    
    fn config_path() -> Result<PathBuf> {
        Ok(dirs::config_dir()
            .context("No config dir")?
            .join("cosmic-desktop-widget")
            .join("config.toml"))
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_update() {
        let mut widget = ClockWidget::new();
        let old_time = widget.time_string();
        std::thread::sleep(Duration::from_secs(1));
        widget.update();
        // Time should have changed
        assert_ne!(old_time, widget.time_string());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml).unwrap();
        assert_eq!(config.width, deserialized.width);
    }
}
```

### Integration Tests

```rust
// tests/layer_shell.rs
#[test]
fn test_buffer_creation() {
    // Test buffer pool creation and retrieval
}

#[test]
fn test_surface_configuration() {
    // Test Layer Shell surface configuration
}
```

## Performance Guidelines

### Memory Usage Targets

- **Idle**: < 20 MB RAM
- **Active**: < 50 MB RAM
- **Buffer Pool**: 2x (width × height × 4 bytes)

### CPU Usage Targets

- **Idle**: < 0.1% CPU
- **Updating**: < 1% CPU
- **Rendering**: < 5% CPU (brief spikes)

### Optimization Checklist

- [ ] Reuse buffers (don't allocate every frame)
- [ ] Only redraw when data changes
- [ ] Use efficient rendering (tiny-skia is already optimized)
- [ ] Minimize allocations in hot paths
- [ ] Profile with `cargo flamegraph`

## Common Development Tasks

### Adding a New Widget

1. Create widget struct in `src/widget/mod.rs`:
```rust
pub struct NewWidget {
    data: String,
    last_update: Instant,
}
```

2. Implement update and display methods:
```rust
impl NewWidget {
    pub fn update(&mut self) { /* ... */ }
    pub fn display_string(&self) -> String { /* ... */ }
}
```

3. Add to main widget struct:
```rust
struct DesktopWidget {
    // ...
    new_widget: NewWidget,
}
```

4. Update rendering in `src/render/mod.rs`

5. Add configuration in `src/config/mod.rs`

### Testing Layer Shell Changes

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Monitor Wayland protocol
WAYLAND_DEBUG=1 cargo run

# Check compositor logs
journalctl --user -u cosmic-comp -f
```

### Debugging Issues

```rust
// Add tracing throughout
use tracing::{debug, info, warn, error};

fn some_function() {
    debug!("Starting function");
    info!("Important event: {}", value);
    warn!("Potential issue: {}", issue);
    error!("Error occurred: {}", err);
}
```

## Troubleshooting Guide

### Widget Doesn't Appear

1. Check Wayland connection: `echo $WAYLAND_DISPLAY`
2. Check Layer Shell support: `weston-info | grep layer_shell`
3. Enable debug logging: `RUST_LOG=debug cargo run`
4. Check compositor logs

### Rendering Issues

1. Verify buffer format (ARGB8888)
2. Check buffer size matches surface size
3. Ensure damage region is correct
4. Profile with `cargo flamegraph`

### Configuration Not Loading

1. Check config path: `~/.config/cosmic-desktop-widget/config.toml`
2. Verify TOML syntax
3. Check file permissions
4. Enable debug logging

## Best Practices Checklist

### Code Quality

- [ ] No `unwrap()` or `expect()` in production code
- [ ] All public functions documented
- [ ] Error types use `thiserror`
- [ ] Logging with `tracing` crate
- [ ] Unit tests for core functionality
- [ ] Integration tests for Wayland interactions
- [ ] Clippy warnings addressed
- [ ] Code formatted with `rustfmt`

### Layer Shell Specific

- [ ] Always check `configured` before drawing
- [ ] Handle configure events properly
- [ ] Clean up surfaces on drop
- [ ] Reuse buffers efficiently
- [ ] Set appropriate keyboard interactivity
- [ ] Use correct layer (Bottom for desktop widgets)
- [ ] Handle compositor closing surface

### Performance

- [ ] Profile before optimizing
- [ ] Avoid allocations in hot paths
- [ ] Reuse buffers
- [ ] Only redraw when needed
- [ ] Use appropriate update intervals

## Resources

### Documentation
- [Layer Shell Specification](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [Smithay Client Toolkit](https://smithay.github.io/client-toolkit/)
- [Wayland Book](https://wayland-book.com/)
- [COSMIC Desktop](https://github.com/pop-os/cosmic-epoch)

### Example Projects
- [Waybar](https://github.com/Alexays/Waybar) - Status bar
- [eww](https://github.com/elkowar/eww) - Widget system
- [Smithay Examples](https://github.com/Smithay/client-toolkit/tree/master/examples)

## Communication with AI Assistants

When asking for help:

1. **Provide context**: "I'm implementing a new widget for the Layer Shell desktop widget project"
2. **Reference this file**: "According to CLAUDE.md, what's the pattern for..."
3. **Include relevant skill**: "Based on the layer_shell_skill.md, how do I..."
4. **Show code**: Include the actual code you're working on
5. **Describe the issue**: "Widget doesn't render" vs "Buffer size mismatch in Layer Shell surface"

## Project Values

1. **Native Wayland** - No X11 compatibility layers
2. **Proper protocols** - Use Layer Shell, not workarounds
3. **Clean code** - Readable, maintainable, documented
4. **Performance** - Lightweight, efficient resource usage
5. **Extensible** - Easy to add new widgets
6. **User-friendly** - Simple configuration, good defaults

---

**Last Updated**: 2025-01-13  
**Project Phase**: Active Development  
**Focus**: Layer Shell implementation, widget system
