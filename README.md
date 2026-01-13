# COSMIC Desktop Widget 🎨

**True desktop widgets using Wayland Layer Shell protocol**

A desktop widget system for COSMIC Desktop that uses the Wayland Layer Shell protocol to display widgets directly on your desktop background - like KDE Plasma widgets or Windows desktop gadgets.

## 🌟 Features

- ✅ **True Desktop Widgets** - Lives on desktop background using Layer Shell
- ✅ **Clock Widget** - Real-time clock with customizable format
- ✅ **Weather Widget** - Current weather conditions (OpenWeatherMap)
- ✅ **Configurable** - Position, size, and appearance
- ✅ **Native Wayland** - No X11, pure Wayland implementation
- ✅ **Lightweight** - Written in Rust with minimal dependencies
- ✅ **COSMIC Integration** - Designed for COSMIC Desktop Environment

## 🖼️ What It Looks Like

```
┌──────────────────────────────────┐
│  14:35:22                        │
│  London                          │
│  22°C Sunny | 65% humidity       │
│  ───────────────────────────────  │
└──────────────────────────────────┘
```

This widget floats on your desktop background, positioned where you want it.

## 🏗️ Architecture

This is a **Layer Shell widget**, not a panel applet:

```
Desktop Layer Stack:
┌─────────────────────────────┐
│  Overlay Layer              │  <- Lock screens, notifications
├─────────────────────────────┤
│  Top Layer                  │  <- On-screen displays
├─────────────────────────────┤
│  Regular Windows            │  <- Your applications
├─────────────────────────────┤
│  Bottom Layer               │  <- Our widget lives here! ⭐
├─────────────────────────────┤
│  Background Layer           │  <- Wallpaper
└─────────────────────────────┘
```

## 📋 Requirements

### System Requirements
- **Compositor**: COSMIC Desktop or any Wayland compositor with Layer Shell support
  - COSMIC ✅
  - Sway ✅
  - Hyprland ✅
  - River ✅
  - GNOME ❌ (no Layer Shell support)
  - KDE Plasma ✅

### Build Requirements (NixOS)
- NixOS with flakes enabled
- Wayland session running

### Build Requirements (Other distros)
- Rust 1.75+
- Wayland development libraries
- pkg-config

## 🚀 Quick Start (NixOS)

```bash
# Clone the repository
git clone <your-repo-url>
cd cosmic-desktop-widget

# Enter development shell
nix develop

# Check your system supports Layer Shell
just check-system

# Create default configuration
just create-config

# Edit config (add weather API key!)
nano ~/.config/cosmic-desktop-widget/config.toml

# Build and run
just build
just run
```

## 🔧 Configuration

Configuration file: `~/.config/cosmic-desktop-widget/config.toml`

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

# Weather settings
weather_city = "London"
weather_api_key = "YOUR_API_KEY_HERE"  # Get from https://openweathermap.org/api

# Update interval in seconds
update_interval = 600  # 10 minutes

# Display options
show_clock = true
show_weather = true

# Clock format: "12h" or "24h"
clock_format = "24h"

# Temperature unit: "celsius" or "fahrenheit"
temperature_unit = "celsius"
```

### Getting Weather API Key

1. Go to https://openweathermap.org/api
2. Sign up for free account
3. Get your API key
4. Add it to config file

## 📖 Usage

```bash
# Run the widget
just run

# Run with debug logging
just run-debug

# Check system compatibility
just check-system

# View current config
just show-config

# Install system-wide
just install
```

## 🎨 Customization

### Position

Choose where the widget appears:
- `top-left` - Upper left corner
- `top-right` - Upper right corner (default)
- `bottom-left` - Lower left corner
- `bottom-right` - Lower right corner
- `center` - Center of screen

### Size

Adjust dimensions in config:
```toml
width = 500   # Wider widget
height = 200  # Taller widget
```

### Margins

Control distance from screen edges:
```toml
[margin]
top = 50      # 50 pixels from top
right = 100   # 100 pixels from right
bottom = 0
left = 0
```

## 🔍 How It Works

### Layer Shell Protocol

This widget uses the `zwlr_layer_shell_v1` Wayland protocol:

1. **Connects to Wayland** - Establishes connection to compositor
2. **Creates Layer Surface** - Requests a surface on the "bottom" layer
3. **Configures Position** - Sets anchor point and margins
4. **Renders Content** - Draws widget using shared memory buffers
5. **Updates Periodically** - Redraws on timer (clock) or interval (weather)

### Technology Stack

- **Wayland**: `smithay-client-toolkit` - Client-side Wayland protocol handling
- **Rendering**: `tiny-skia` - 2D graphics rendering
- **Event Loop**: `calloop` - Async event loop for Wayland events
- **Weather**: `reqwest` - HTTP client for OpenWeatherMap API
- **Config**: `toml` - Configuration file parsing

## 🐛 Troubleshooting

### Widget Doesn't Appear

```bash
# Check Wayland is running
echo $WAYLAND_DISPLAY  # Should output something like "wayland-0"

# Check Layer Shell support
just check-layer-shell

# Run with debug logging
RUST_LOG=debug just run
```

### Weather Not Showing

1. Check API key is set in config
2. Check internet connection
3. View logs for errors:
   ```bash
   RUST_LOG=debug just run
   ```

### Widget Position Wrong

1. Check compositor supports Layer Shell anchors
2. Try different position in config:
   ```toml
   position = "top-left"  # Try different corner
   ```

### Compositor Not Supported

Layer Shell is supported by:
- ✅ COSMIC Desktop
- ✅ Sway
- ✅ Hyprland
- ✅ River
- ❌ GNOME (Mutter doesn't support Layer Shell)

## 🏗️ Development

### Project Structure

```
cosmic-desktop-widget/
├── src/
│   ├── main.rs           # Entry point, Layer Shell setup
│   ├── wayland/          # Wayland buffer management
│   │   └── mod.rs
│   ├── render/           # Rendering with tiny-skia
│   │   └── mod.rs
│   ├── widget/           # Widget implementations
│   │   └── mod.rs
│   └── config/           # Configuration management
│       └── mod.rs
├── Cargo.toml            # Dependencies
├── flake.nix             # NixOS build configuration
├── justfile              # Build automation
└── README.md             # This file
```

### Building from Source

```bash
# With Nix
nix develop
cargo build --release

# Without Nix (install dependencies first)
cargo build --release
```

### Adding New Widgets

1. Create widget struct in `src/widget/mod.rs`
2. Implement `update()` and `display_string()` methods
3. Add to renderer in `src/render/mod.rs`
4. Update config in `src/config/mod.rs`

### Testing

```bash
# Run tests
just test

# Run with logging
RUST_LOG=trace just run

# Check code quality
just check-all
```

## 📚 Resources

### Wayland Layer Shell
- [Protocol Specification](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [Smithay Client Toolkit Docs](https://smithay.github.io/client-toolkit/)

### COSMIC Desktop
- [COSMIC GitHub](https://github.com/pop-os/cosmic-epoch)
- [libcosmic](https://github.com/pop-os/libcosmic)

### Similar Projects
- [Waybar](https://github.com/Alexays/Waybar) - Status bar using Layer Shell
- [eww](https://github.com/elkowar/eww) - Widget system for Wayland

## 🤝 Contributing

Contributions welcome! This is a proof-of-concept showing how to build desktop widgets with Layer Shell.

Areas for improvement:
- Better text rendering (fontdue, rusttype)
- More widget types (system monitor, calendar, todo list)
- Click interaction support
- Theme integration with COSMIC
- Configuration GUI

## 📝 License

GPL-3.0 - See LICENSE file

## 🙏 Acknowledgments

- System76 for COSMIC Desktop
- Smithay project for Wayland libraries
- Wayland compositor developers

---

**Built with ❤️ using Rust and Wayland Layer Shell**

For questions or issues, please open a GitHub issue!
