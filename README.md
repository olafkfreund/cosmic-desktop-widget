# COSMIC Desktop Widget

A lightweight, configurable desktop widget system for COSMIC Desktop Environment using the Wayland Layer Shell protocol. Widgets live directly on your desktop background - below windows, above wallpaper - just like KDE Plasma widgets or classic Windows desktop gadgets.

## Features

- **True Desktop Widgets** - Uses Layer Shell protocol (`zwlr_layer_shell_v1`) to render on desktop background
- **Clock Widget** - Configurable 12h/24h format with optional seconds and date display
- **Weather Widget** - OpenWeatherMap integration with Celsius/Fahrenheit support
- **Theming System** - Three built-in themes (cosmic_dark, light, transparent_dark) plus custom themes
- **Performance Optimized** - Glyph caching, smart update scheduling, metrics tracking
- **Flexible Layout** - Vertical/horizontal widget arrangement with configurable padding/spacing
- **Native Wayland** - Pure Wayland implementation using Smithay Client Toolkit

## How It Works

```
Desktop Layer Stack:
+-----------------------------+
|  Overlay Layer              |  <- Lock screens, notifications
+-----------------------------+
|  Top Layer                  |  <- On-screen displays
+-----------------------------+
|  Regular Windows            |  <- Your applications
+-----------------------------+
|  Bottom Layer               |  <- OUR WIDGET LIVES HERE
+-----------------------------+
|  Background Layer           |  <- Wallpaper
+-----------------------------+
```

## Requirements

### System Requirements

- Linux with Wayland compositor supporting Layer Shell protocol
- Supported compositors:
  - COSMIC Desktop (recommended)
  - Sway
  - Hyprland
  - River
  - KDE Plasma (Wayland session)
  - **Not supported:** GNOME (Mutter lacks Layer Shell support)

### Build Requirements

- Rust 1.75 or later
- Wayland development libraries
- pkg-config
- fontconfig and freetype

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-username/cosmic-desktop-widget
cd cosmic-desktop-widget

# Build release binary
cargo build --release

# Copy to a location in your PATH
sudo cp target/release/cosmic-desktop-widget /usr/local/bin/
```

### NixOS / Nix

Add to your flake inputs:

```nix
{
  inputs = {
    cosmic-desktop-widget = {
      url = "github:your-username/cosmic-desktop-widget";
    };
  };
}
```

Then include in your packages:

```nix
environment.systemPackages = [
  inputs.cosmic-desktop-widget.packages.${system}.default
];
```

Or use the development shell:

```bash
nix develop
cargo build --release
```

### Building with Just

If you have `just` installed:

```bash
just build          # Build debug version
just build-release  # Build release version
just run            # Build and run
just test           # Run test suite
just check          # Run clippy checks
```

## Configuration

Configuration is stored in `~/.config/cosmic-desktop-widget/config.toml`. A default configuration is created on first run.

### Basic Configuration

```toml
# Widget dimensions
width = 400
height = 150

# Position: "top-left", "top-right", "bottom-left", "bottom-right", "center"
position = "top-right"

# Margins from screen edges
[margin]
top = 20
right = 20
bottom = 0
left = 0

# Clock settings
show_clock = true
clock_format = "24h"      # "24h" or "12h"
show_seconds = true
show_date = false

# Weather settings
show_weather = true
weather_city = "London"
weather_api_key = ""      # Get from https://openweathermap.org/api
temperature_unit = "celsius"  # "celsius" or "fahrenheit"
update_interval = 600     # Weather update interval in seconds

# Theme: "cosmic_dark", "light", "transparent_dark", or "custom"
theme = "cosmic_dark"

# Layout settings
padding = 20.0
spacing = 10.0
```

### Getting a Weather API Key

1. Visit [OpenWeatherMap](https://openweathermap.org/api)
2. Create a free account
3. Generate an API key
4. Add it to your config file

### Custom Theme

To create a custom theme, set `theme = "custom"` and add:

```toml
theme = "custom"

[custom_theme]
opacity = 0.9
border_width = 2.0
corner_radius = 8.0

[custom_theme.background]
r = 30
g = 30
b = 30
a = 230

[custom_theme.border]
r = 100
g = 100
b = 100
a = 255

[custom_theme.text_primary]
r = 255
g = 255
b = 255
a = 255

[custom_theme.text_secondary]
r = 180
g = 180
b = 180
a = 255

[custom_theme.accent]
r = 52
g = 120
b = 246
a = 255
```

## Usage

### Running the Widget

```bash
# Run with default settings
cosmic-desktop-widget

# Run with info logging
RUST_LOG=info cosmic-desktop-widget

# Run with debug logging (verbose)
RUST_LOG=debug cosmic-desktop-widget

# Run with trace logging (very verbose)
RUST_LOG=trace cosmic-desktop-widget
```

### Autostart

To start the widget automatically:

**Systemd user service:**

Create `~/.config/systemd/user/cosmic-desktop-widget.service`:

```ini
[Unit]
Description=COSMIC Desktop Widget
After=graphical-session.target

[Service]
ExecStart=/usr/local/bin/cosmic-desktop-widget
Restart=on-failure

[Install]
WantedBy=graphical-session.target
```

Then enable it:

```bash
systemctl --user enable --now cosmic-desktop-widget
```

## Architecture

```
cosmic-desktop-widget/
+-- src/
|   +-- main.rs              # Entry point, Layer Shell setup, event loop
|   +-- lib.rs               # Library exports
|   +-- config/mod.rs        # Configuration loading and validation
|   +-- theme/mod.rs         # Theme definitions and color handling
|   +-- widget/mod.rs        # Clock and Weather widget implementations
|   +-- render/mod.rs        # tiny-skia based rendering pipeline
|   +-- layout/mod.rs        # Widget positioning and layout calculations
|   +-- text/                # Text rendering with fontdue
|   |   +-- mod.rs
|   |   +-- font.rs          # Font loading
|   |   +-- renderer.rs      # Text rendering
|   |   +-- glyph_cache.rs   # Glyph caching for performance
|   +-- wayland/mod.rs       # Buffer pool and shared memory management
|   +-- update/mod.rs        # Smart update scheduling
|   +-- metrics/mod.rs       # Performance metrics tracking
|   +-- weather/mod.rs       # Weather API integration
|   +-- error.rs             # Error types
+-- tests/
|   +-- integration_tests.rs # Integration test suite
+-- docs/
|   +-- CONFIGURATION.md     # Detailed configuration reference
|   +-- ARCHITECTURE.md      # System architecture documentation
+-- Cargo.toml               # Dependencies
+-- flake.nix                # NixOS build configuration
+-- justfile                 # Build automation
+-- CHANGELOG.md             # Version history
```

## Troubleshooting

### Widget Does Not Appear

1. **Check Wayland is running:**
   ```bash
   echo $WAYLAND_DISPLAY  # Should output "wayland-0" or similar
   ```

2. **Check Layer Shell support:**
   ```bash
   # Look for zwlr_layer_shell in protocol list
   wayland-info | grep layer_shell
   ```

3. **Run with debug logging:**
   ```bash
   RUST_LOG=debug cosmic-desktop-widget
   ```

### Weather Not Showing

1. Verify API key is set in config
2. Check internet connection
3. Verify city name is correct (use English names)
4. Check logs for API errors:
   ```bash
   RUST_LOG=debug cosmic-desktop-widget 2>&1 | grep -i weather
   ```

### Widget Position Wrong

1. Check `position` value is one of: `top-left`, `top-right`, `bottom-left`, `bottom-right`, `center`
2. Verify margins are correct in config
3. Some compositors may handle Layer Shell anchoring differently

### High CPU Usage

1. Check update interval is not too low (minimum recommended: 60 seconds)
2. Enable metrics logging to diagnose:
   ```bash
   RUST_LOG=debug cosmic-desktop-widget
   ```
3. Performance metrics are logged every 60 seconds

### Font Not Rendering

1. Ensure system fonts are installed (DejaVu Sans, Liberation Sans, or Noto Sans)
2. Check fontconfig is properly configured
3. Run with trace logging to see font loading:
   ```bash
   RUST_LOG=trace cosmic-desktop-widget 2>&1 | grep -i font
   ```

## Performance

### Target Metrics

| Metric | Target | Description |
|--------|--------|-------------|
| Idle CPU | < 0.1% | When display is static (no updates) |
| Active CPU | < 1% | During widget updates (clock tick) |
| Idle RAM | < 20 MB | Baseline memory usage |
| Active RAM | < 50 MB | With all widgets enabled |
| Render time | < 5 ms | Time to draw a single frame |
| Frame budget | < 16 ms | Total time per frame (60fps) |

### Performance Optimizations

The widget system implements several optimizations to minimize resource usage:

**Rendering Optimizations:**
- Dirty region tracking - only redraws when content actually changes
- Cached font size calculations - avoids binary search on every frame
- Glyph caching - reuses rasterized text glyphs across frames
- Conditional buffer clearing - skips clearing when not needed

**Update Optimizations:**
- Dynamic timer intervals - sleeps until next widget needs updating
- Change detection - compares current vs. previous state before redrawing
- Cached time strings - avoids formatting on every frame

**Memory Optimizations:**
- Efficient buffer pool with double-buffering
- Borrowed string references where possible
- Reused glyph bitmaps (no cloning in render loop)

### Profiling

```bash
# Run with performance metrics logging
RUST_LOG=debug cargo run --release

# Generate CPU flamegraph (requires cargo-flamegraph)
cargo flamegraph --bin cosmic-desktop-widget

# Memory profiling with heaptrack
heaptrack ./target/release/cosmic-desktop-widget

# Benchmark specific operations
cargo bench
```

### Benchmark Results

Run benchmarks with `cargo bench`. Results are saved to `target/criterion/`.

Key benchmarks:
- `clock_update` - Time to update clock widget
- `clock_time_string/borrow` - Optimized string access
- `scheduler_check_updates` - Update scheduling overhead
- `theme_background_with_opacity` - Color operations

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_config_round_trip
```

### Code Quality

```bash
# Run clippy
cargo clippy

# Format code
cargo fmt

# Check all
cargo clippy && cargo fmt --check && cargo test
```

### Adding New Widgets

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for details on the widget system and how to extend it.

## Documentation

- [Configuration Reference](docs/CONFIGURATION.md) - Detailed documentation of all configuration options
- [Architecture Guide](docs/ARCHITECTURE.md) - System design and implementation details
- [Changelog](CHANGELOG.md) - Version history and release notes

## Resources

### Wayland Layer Shell

- [Protocol Specification](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [Smithay Client Toolkit](https://smithay.github.io/client-toolkit/)
- [Wayland Book](https://wayland-book.com/)

### COSMIC Desktop

- [COSMIC Desktop](https://github.com/pop-os/cosmic-epoch)
- [libcosmic](https://github.com/pop-os/libcosmic)

### Similar Projects

- [Waybar](https://github.com/Alexays/Waybar) - Status bar using Layer Shell
- [eww](https://github.com/elkowar/eww) - Widget system for Wayland

## Contributing

Contributions are welcome! Please read the existing code and follow the established patterns.

Areas for improvement:

- Additional widget types (system monitor, calendar, todo list)
- Click/touch interaction support
- Multi-monitor support
- Configuration GUI
- Dynamic theming from COSMIC settings

## License

GPL-3.0 - See [LICENSE](LICENSE) file for details.

## Acknowledgments

- System76 for COSMIC Desktop Environment
- Smithay project for excellent Wayland libraries
- tiny-skia for efficient 2D rendering
- fontdue for high-quality font rasterization
