# Layer Shell Development Guide

## What is Layer Shell?

The Wayland Layer Shell protocol (`zwlr_layer_shell_v1`) allows applications to create surfaces that exist in specific "layers" of the desktop, rather than as regular windows.

### Layer Stack

```
Overlay     - Lock screens, critical notifications
  ↓
Top         - On-screen displays, tooltips
  ↓
Normal      - Regular application windows
  ↓
Bottom      - Desktop widgets (OUR LAYER!)
  ↓
Background  - Wallpaper
```

## Why Use Layer Shell?

### Advantages

1. **Proper Z-ordering** - Widget stays below windows, above wallpaper
2. **No window decorations** - Clean appearance
3. **Fixed positioning** - Anchored to screen edges/corners
4. **Compositor integration** - Works with tiling, fullscreen, etc.
5. **Input passthrough** - Can allow clicks to pass through

### Compared to Regular Windows

| Feature | Regular Window | Layer Shell |
|---------|---------------|-------------|
| Z-order | With other windows | Fixed layer |
| Decorations | Yes (titlebar, borders) | No |
| Positioning | Managed by compositor | Explicit anchoring |
| Tiling | Participates | Ignored |
| Fullscreen | Covers/hides | Stays visible |

## Protocol Basics

### Creating a Layer Surface

```rust
// 1. Get the layer shell global
let layer_shell = LayerShell::bind(&globals, &qh)?;

// 2. Create a Wayland surface
let wl_surface = compositor_state.create_surface(&qh);

// 3. Create layer surface from it
let layer = layer_shell.create_layer_surface(
    &qh,
    wl_surface,
    Layer::Bottom,           // Which layer
    Some("my-widget"),       // Namespace (unique ID)
    None,                    // Output (None = all outputs)
);
```

### Configuring Position

```rust
// Anchor to corner
layer.set_anchor(Anchor::TOP | Anchor::RIGHT);

// Set size
layer.set_size(400, 150);

// Set margins
layer.set_margin(
    20,  // top
    20,  // right
    0,   // bottom
    0,   // left
);

// Keyboard interactivity
layer.set_keyboard_interactivity(KeyboardInteractivity::None);

// Exclusive zone (space reserved for this widget)
layer.set_exclusive_zone(-1);  // -1 = don't reserve space

// Commit configuration
layer.commit();
```

### Anchor Options

```rust
Anchor::TOP              // Top edge
Anchor::BOTTOM           // Bottom edge
Anchor::LEFT             // Left edge
Anchor::RIGHT            // Right edge
Anchor::TOP | Anchor::LEFT     // Top-left corner
Anchor::TOP | Anchor::RIGHT    // Top-right corner
// etc...
```

### Layers

```rust
Layer::Background  // Below everything
Layer::Bottom      // Above background, below windows
Layer::Top         // Above windows
Layer::Overlay     // Above everything
```

### Keyboard Interactivity

```rust
KeyboardInteractivity::None       // No keyboard input
KeyboardInteractivity::OnDemand   // Gets focus when needed
KeyboardInteractivity::Exclusive  // Always has focus (for lock screens)
```

## Rendering

### Shared Memory Buffers

Layer Shell surfaces use Wayland shared memory (wl_shm) for rendering:

```rust
// 1. Create a shared memory pool
let pool = SlotPool::new(size, &shm_state)?;

// 2. Get a buffer from the pool
let (buffer, canvas) = pool.create_buffer(
    width,
    height,
    stride,
    wl_shm::Format::Argb8888,
)?;

// 3. Draw to the canvas (raw pixel data)
for pixel in canvas.chunks_exact_mut(4) {
    pixel[0] = b;  // Blue
    pixel[1] = g;  // Green
    pixel[2] = r;  // Red
    pixel[3] = a;  // Alpha
}

// 4. Attach buffer to surface
surface.attach(Some(&buffer), 0, 0);
surface.damage_buffer(0, 0, width as i32, height as i32);
surface.commit();
```

### Pixel Format

ARGB8888 format (32 bits per pixel):
```
[31:24] Alpha
[23:16] Red
[15:8]  Green
[7:0]   Blue
```

## Event Handling

### Configure Event

The compositor sends configure events:

```rust
impl LayerShellHandler for MyWidget {
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
        
        // Now we can render
        self.draw(qh);
    }
}
```

### Close Event

```rust
fn closed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _layer: &LayerSurface,
) {
    // Widget was closed by compositor
    // Clean up and exit
}
```

## Best Practices

### 1. Don't Block the Event Loop

```rust
// ❌ BAD
fn update(&mut self) {
    std::thread::sleep(Duration::from_secs(1));
    self.fetch_data();  // Blocking network call
}

// ✅ GOOD
fn update(&mut self) {
    // Use async or spawn thread
    let handle = tokio::spawn(async {
        fetch_data().await
    });
}
```

### 2. Handle Configuration

```rust
// ✅ Always check if configured before drawing
if !self.configured {
    return;
}
```

### 3. Efficient Rendering

```rust
// ✅ Only redraw when needed
fn should_redraw(&self) -> bool {
    self.data_changed || self.timer_expired
}

fn draw(&mut self, qh: &QueueHandle<Self>) {
    if !self.should_redraw() {
        return;
    }
    // ... render
}
```

### 4. Clean Up Resources

```rust
// ✅ Drop layer surface properly
impl Drop for MyWidget {
    fn drop(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.destroy();
        }
    }
}
```

## Common Patterns

### Periodic Updates

```rust
// Using calloop timer
let timer = Timer::new()?;
event_loop.handle().insert_source(timer, |_, _, widget| {
    widget.update();
    TimeoutAction::ToDuration(Duration::from_secs(1))
})?;
```

### Async Data Fetching

```rust
// Spawn task, send result via channel
let (tx, rx) = std::sync::mpsc::channel();

std::thread::spawn(move || {
    let data = fetch_weather();
    tx.send(data).ok();
});

// In event loop, check channel
if let Ok(data) = self.rx.try_recv() {
    self.weather_data = Some(data);
    self.draw(qh);
}
```

### Multiple Outputs

```rust
// Show widget on specific output
let output = output_state.outputs().next().unwrap();

let layer = layer_shell.create_layer_surface(
    &qh,
    surface,
    Layer::Bottom,
    Some("widget"),
    Some(&output),  // Specific output
);
```

## Debugging

### Enable Protocol Logging

```bash
WAYLAND_DEBUG=1 ./cosmic-desktop-widget
```

### Check Layer Shell Support

```bash
# Install wayland-utils
nix-shell -p wayland-utils

# Check protocols
weston-info | grep layer_shell
```

### Monitor Wayland Events

```bash
# Install wlgreet
wlgreet --help

# Or use wayland-debug
wayland-debug ./cosmic-desktop-widget
```

## Compositor Compatibility

### COSMIC Desktop ✅

Full support, tested and working.

### Sway ✅

Excellent Layer Shell support:
```
layer {
    default top
}
```

### Hyprland ✅

Good support with configuration:
```
layerrule = noanim, cosmic-desktop-widget
```

### GNOME ❌

Mutter doesn't support Layer Shell protocol.

### KDE Plasma ✅

Partial support, may need configuration.

## Performance Tips

### 1. Buffer Management

```rust
// ✅ Reuse buffers
struct BufferPool {
    buffers: Vec<(WlBuffer, Vec<u8>)>,
    current: usize,
}

impl BufferPool {
    fn next_buffer(&mut self) -> (&WlBuffer, &mut [u8]) {
        self.current = (self.current + 1) % self.buffers.len();
        let (buf, data) = &mut self.buffers[self.current];
        (buf, data)
    }
}
```

### 2. Partial Updates

```rust
// ✅ Only damage changed regions
surface.damage_buffer(
    x, y,        // Changed region
    width, height
);
```

### 3. Optimize Rendering

```rust
// ✅ Use fast rendering libraries
// tiny-skia for 2D graphics
// femtovg for hardware-accelerated rendering
```

## Resources

- [Layer Shell Protocol Spec](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit)
- [Wayland Book](https://wayland-book.com/)
- [Example: Waybar](https://github.com/Alexays/Waybar)

---

**Happy Layer Shell Development!** 🚀
