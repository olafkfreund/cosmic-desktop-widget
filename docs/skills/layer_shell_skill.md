# Wayland Layer Shell Skill

## Overview

The Wayland Layer Shell protocol (`zwlr_layer_shell_v1`) allows applications to create surfaces at specific "layers" of the desktop stack, enabling desktop shell components like panels, notifications, and desktop widgets.

This skill covers practical implementation using the Smithay Client Toolkit in Rust.

## Protocol Basics

### Layer Stack

```
Overlay Layer     ← Lock screens, critical notifications
Top Layer         ← OSD, tooltips, popups
Normal Windows    ← Regular applications (XDG Shell)
Bottom Layer      ← Desktop widgets (OUR TARGET)
Background Layer  ← Wallpaper
```

### Core Concepts

**Layer Surface** - A Wayland surface assigned to a specific layer
**Anchor** - How the surface attaches to screen edges
**Exclusive Zone** - Space reserved for the surface
**Keyboard Interactivity** - How the surface receives keyboard input

## Implementation with Smithay

### Dependencies

```toml
[dependencies]
smithay-client-toolkit = { version = "0.18", features = ["calloop"] }
wayland-client = "0.31"
wayland-protocols-wlr = { version = "0.2", features = ["client"] }
calloop = "0.12"
```

### Initialization

```rust
use smithay_client_toolkit::{
    compositor::CompositorState,
    shell::wlr_layer::{LayerShell, Layer, Anchor, KeyboardInteractivity},
    shm::Shm,
};
use wayland_client::{Connection, globals::registry_queue_init};

// 1. Connect to Wayland
let conn = Connection::connect_to_env()
    .expect("Failed to connect to Wayland");

// 2. Initialize registry
let (globals, event_queue) = registry_queue_init(&conn)
    .expect("Failed to initialize registry");

let qh = event_queue.handle();

// 3. Bind required globals
let compositor = CompositorState::bind(&globals, &qh)
    .expect("wl_compositor not available");
    
let layer_shell = LayerShell::bind(&globals, &qh)
    .expect("layer_shell not available");
    
let shm = Shm::bind(&globals, &qh)
    .expect("wl_shm not available");
```

### Creating a Layer Surface

```rust
// Create base Wayland surface
let wl_surface = compositor.create_surface(&qh);

// Create Layer Shell surface
let layer = layer_shell.create_layer_surface(
    &qh,
    wl_surface,
    Layer::Bottom,           // Which layer
    Some("my-widget"),       // Namespace (unique ID)
    None,                    // Output (None = all outputs)
);

// Configure the surface
layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
layer.set_size(400, 150);
layer.set_margin(20, 20, 0, 0);  // top, right, bottom, left
layer.set_keyboard_interactivity(KeyboardInteractivity::None);
layer.set_exclusive_zone(-1);     // -1 = don't reserve space

// Commit configuration
layer.commit();
```

### Anchor Positions

```rust
// Corners
Anchor::TOP | Anchor::LEFT       // Top-left
Anchor::TOP | Anchor::RIGHT      // Top-right
Anchor::BOTTOM | Anchor::LEFT    // Bottom-left
Anchor::BOTTOM | Anchor::RIGHT   // Bottom-right

// Edges
Anchor::TOP                      // Top edge (centered)
Anchor::BOTTOM                   // Bottom edge (centered)
Anchor::LEFT                     // Left edge (centered)
Anchor::RIGHT                    // Right edge (centered)

// Center
Anchor::empty()                  // Center of screen

// Stretch
Anchor::TOP | Anchor::BOTTOM     // Full height
Anchor::LEFT | Anchor::RIGHT     // Full width
Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT  // Fullscreen
```

### Layer Types

```rust
Layer::Background  // Wallpaper layer (lowest)
Layer::Bottom      // Desktop widgets (above wallpaper)
Layer::Top         // Panels, bars (above windows)
Layer::Overlay     // Lock screens, critical notifications (highest)
```

### Keyboard Interactivity

```rust
KeyboardInteractivity::None       // No keyboard input (widgets)
KeyboardInteractivity::OnDemand   // Focus when clicked (menus)
KeyboardInteractivity::Exclusive  // Always focused (lock screens)
```

### Exclusive Zone

```rust
// Reserve space (panel behavior)
layer.set_exclusive_zone(50);     // Reserve 50 pixels

// Don't reserve space (widget behavior)
layer.set_exclusive_zone(-1);     // -1 = no reservation

// Auto-calculate based on size
layer.set_exclusive_zone(0);      // 0 = auto
```

## Event Handling

### Implementing LayerShellHandler

```rust
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};

struct MyWidget {
    layer: Option<LayerSurface>,
    width: u32,
    height: u32,
    configured: bool,
}

impl LayerShellHandler for MyWidget {
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Compositor may change our size
        if configure.new_size.0 > 0 && configure.new_size.1 > 0 {
            self.width = configure.new_size.0;
            self.height = configure.new_size.1;
        }
        
        // Mark as configured
        self.configured = true;
        
        // Now we can render
        self.draw(qh);
    }

    fn closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
    ) {
        // Compositor closed our surface
        tracing::info!("Layer surface closed");
        self.layer = None;
    }
}

// Register the delegate
delegate_layer!(MyWidget);
```

### Important: Wait for Configure

```rust
fn draw(&mut self, qh: &QueueHandle<Self>) {
    // ✅ ALWAYS check if configured first
    if !self.configured {
        return;
    }
    
    // Now safe to render
    // ...
}
```

## Rendering Pipeline

### Buffer Creation

```rust
use smithay_client_toolkit::shm::slot::SlotPool;

// Create buffer pool
let stride = width * 4;  // ARGB8888 = 4 bytes per pixel
let size = (stride * height) as usize;

let mut pool = SlotPool::new(size * 2, &shm)  // Double buffering
    .expect("Failed to create pool");

// Get a buffer
let (buffer, canvas) = pool.create_buffer(
    width as i32,
    height as i32,
    stride as i32,
    wl_shm::Format::Argb8888,
)?;
```

### Drawing to Canvas

```rust
// ARGB8888 format: [Blue, Green, Red, Alpha]
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = blue;   // B
    pixel[1] = green;  // G
    pixel[2] = red;    // R
    pixel[3] = alpha;  // A (255 = opaque, 0 = transparent)
}
```

### Committing the Surface

```rust
let wl_surface = layer.wl_surface();

// Damage the entire surface (mark as changed)
wl_surface.damage_buffer(0, 0, width as i32, height as i32);

// Attach the buffer
wl_surface.attach(Some(&buffer), 0, 0);

// Commit changes
wl_surface.commit();
```

### Frame Callbacks

```rust
// Request next frame callback
let wl_surface = layer.wl_surface();
wl_surface.frame(&qh, wl_surface.clone());
wl_surface.commit();

// Handle frame callback in CompositorHandler
impl CompositorHandler for MyWidget {
    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Time to render next frame
        self.draw(qh);
    }
}
```

## Best Practices

### 1. Resource Management

```rust
// ✅ Clean up on drop
impl Drop for MyWidget {
    fn drop(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.destroy();
        }
    }
}
```

### 2. Efficient Rendering

```rust
// ✅ Only redraw when needed
struct MyWidget {
    needs_redraw: bool,
}

impl MyWidget {
    fn draw(&mut self, qh: &QueueHandle<Self>) {
        if !self.configured || !self.needs_redraw {
            return;
        }
        
        // Render...
        
        self.needs_redraw = false;
    }
    
    fn update_data(&mut self, new_data: Data) {
        self.data = new_data;
        self.needs_redraw = true;
    }
}
```

### 3. Handle Size Changes

```rust
fn configure(&mut self, configure: LayerSurfaceConfigure, ...) {
    let old_size = (self.width, self.height);
    let new_size = configure.new_size;
    
    if new_size.0 > 0 && new_size.1 > 0 {
        self.width = new_size.0;
        self.height = new_size.1;
    }
    
    // Recreate buffers if size changed
    if (self.width, self.height) != old_size {
        self.buffer_pool = None;  // Will be recreated
    }
    
    self.draw(qh);
}
```

### 4. Multiple Outputs

```rust
// Get all outputs
let outputs = output_state.outputs();

// Create surface on specific output
for output in outputs {
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Bottom,
        Some("widget"),
        Some(&output),  // Specific output
    );
    // Configure...
}
```

## Common Patterns

### Periodic Updates

```rust
// Using calloop timer
use calloop::timer::Timer;

let timer = Timer::new()?;
let handle = timer.handle();

event_loop.handle().insert_source(timer, |_, _, widget| {
    widget.update();
    widget.needs_redraw = true;
    TimeoutAction::ToDuration(Duration::from_secs(1))
})?;

// Trigger first timeout
handle.add_timeout(Duration::from_secs(1), ());
```

### Transparent Background

```rust
// Fill with transparent pixels
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = 0;   // B
    pixel[1] = 0;   // G
    pixel[2] = 0;   // R
    pixel[3] = 0;   // A = 0 (fully transparent)
}
```

### Semi-Transparent Widget

```rust
// Semi-transparent black background
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = 0;    // B
    pixel[1] = 0;    // G
    pixel[2] = 0;    // R
    pixel[3] = 200;  // A = 200 (semi-transparent)
}
```

## Debugging

### Enable Wayland Protocol Logging

```bash
WAYLAND_DEBUG=1 ./my-widget
```

### Check Layer Shell Support

```bash
# Install wayland-utils
weston-info | grep zwlr_layer_shell

# Should show:
# zwlr_layer_shell_v1 (version 4)
```

### Common Issues

**Widget doesn't appear:**
- Check `configured` flag before drawing
- Verify Layer Shell protocol is supported
- Check surface size is > 0
- Ensure commit() is called

**Widget in wrong position:**
- Check anchor configuration
- Verify margin values
- Check compositor behavior

**Widget covers windows:**
- Should use `Layer::Bottom` not `Layer::Top`
- Check compositor's layer implementation

## Compositor Compatibility

### Full Support
- ✅ COSMIC Desktop
- ✅ Sway
- ✅ Hyprland
- ✅ River
- ✅ Wayfire

### Partial Support
- ⚠️ KDE Plasma (kwin_wayland) - Limited
- ⚠️ Labwc - Basic support

### No Support
- ❌ GNOME (Mutter) - Doesn't implement Layer Shell
- ❌ Any X11 environment

## Performance Tips

### 1. Buffer Reuse

```rust
struct BufferPool {
    buffers: Vec<(WlBuffer, Vec<u8>)>,
    current: usize,
}

impl BufferPool {
    fn next(&mut self) -> (&WlBuffer, &mut [u8]) {
        self.current = (self.current + 1) % self.buffers.len();
        // Return existing buffer
    }
}
```

### 2. Partial Damage

```rust
// Only damage changed region
wl_surface.damage_buffer(x, y, width, height);
```

### 3. Throttle Updates

```rust
let last_draw = Instant::now();
if last_draw.elapsed() < Duration::from_millis(16) {
    return;  // Don't redraw too frequently
}
```

## Testing

### Test Surface Creation

```rust
#[test]
fn test_layer_surface_creation() {
    // Connect to Wayland
    // Create layer surface
    // Verify configuration
}
```

### Test Event Handling

```rust
#[test]
fn test_configure_event() {
    // Simulate configure event
    // Verify size update
    // Verify configured flag
}
```

## Resources

- [Protocol Spec](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [Smithay Toolkit](https://smithay.github.io/client-toolkit/)
- [Example: Waybar](https://github.com/Alexays/Waybar)
- [Wayland Book](https://wayland-book.com/)

---

**Key Takeaways:**
1. Always wait for configure before drawing
2. Use Layer::Bottom for desktop widgets
3. Set keyboard interactivity to None for non-interactive widgets
4. Reuse buffers for performance
5. Clean up resources on drop

**Last Updated**: 2025-01-13
