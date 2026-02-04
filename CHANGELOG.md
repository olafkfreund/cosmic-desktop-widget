# Changelog

All notable changes to COSMIC Desktop Widget will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Multi-monitor support
- Click/touch interaction support
- System monitor widget
- Calendar widget
- Configuration GUI
- Dynamic COSMIC theme integration

## [0.1.0] - 2025-02-04

### Added

**Core Features**
- Initial release of COSMIC Desktop Widget
- Wayland Layer Shell integration (`zwlr_layer_shell_v1` protocol)
- True desktop widget positioning (Bottom layer - below windows, above wallpaper)
- Support for COSMIC Desktop, Sway, Hyprland, River, and KDE Plasma compositors

**Clock Widget**
- Real-time clock display
- Configurable 12-hour and 24-hour format
- Optional seconds display
- Optional date display (full date format: "Monday, January 15, 2025")
- Automatic 1-second update interval

**Weather Widget**
- OpenWeatherMap API integration
- Current temperature display
- Weather condition (Sunny, Cloudy, Rain, etc.)
- Humidity percentage
- Celsius and Fahrenheit support with automatic conversion
- Configurable update interval (default: 10 minutes)
- Error handling with stale data indicators
- Graceful degradation when API unavailable

**Theming System**
- Three built-in themes:
  - `cosmic_dark` - Dark theme matching COSMIC Desktop aesthetic
  - `light` - Light theme for bright environments
  - `transparent_dark` - Semi-transparent dark theme
- Full custom theme support via configuration
- Theme properties: background, border, text colors, accent, opacity, border width, corner radius

**Configuration System**
- TOML configuration file (`~/.config/cosmic-desktop-widget/config.toml`)
- Auto-creation of default configuration on first run
- Configuration validation with helpful error messages
- Graceful fallback to defaults on invalid configuration

**Layout System**
- Flexible widget positioning (top-left, top-right, bottom-left, bottom-right, center)
- Configurable margins from screen edges
- Configurable internal padding and widget spacing
- Vertical and horizontal layout support

**Performance**
- Glyph caching for efficient text rendering
- Smart update scheduling (only redraw when data changes)
- Double-buffered Wayland surfaces
- Performance metrics tracking and logging
- Target: <16ms render time (60fps budget)
- Target: <0.1% idle CPU, <1% active CPU, <50MB memory

**Text Rendering**
- fontdue-based high-quality text rasterization
- LRU glyph cache to avoid repeated rasterization
- System font loading via fontconfig
- Fallback font chain (DejaVu Sans, Liberation Sans, Noto Sans, FreeSans)

**Developer Experience**
- Comprehensive test suite (64+ tests)
- Unit tests for all core components
- Integration tests for component interaction
- Nix flake for reproducible builds and development environment
- justfile for common development tasks
- Detailed logging with configurable levels (RUST_LOG)

**Documentation**
- Comprehensive README with installation and usage instructions
- Detailed configuration reference (docs/CONFIGURATION.md)
- Architecture documentation (docs/ARCHITECTURE.md)
- AI assistant skills for development (docs/skills/)

### Technical Details

**Dependencies**
- Rust 1.75+ (2021 edition)
- smithay-client-toolkit 0.18 (Wayland client library)
- wayland-client 0.31 (Wayland protocol)
- wayland-protocols-wlr 0.2 (Layer Shell protocol)
- calloop 0.12 (Event loop)
- tiny-skia 0.11 (2D rendering)
- fontdue 0.9 (Font rasterization)
- reqwest 0.12 (HTTP client for weather API)
- tokio 1 (Async runtime)
- chrono 0.4 (Time handling)
- serde 1 + toml 0.8 (Configuration)
- tracing 0.1 (Logging)
- thiserror 1 + anyhow 1 (Error handling)

**Build Configuration**
- Release profile: LTO enabled, single codegen unit, stripped binary
- NixOS flake with development shell
- GPL-3.0 license

### Known Limitations

- GNOME/Mutter not supported (no Layer Shell protocol)
- No click/touch interaction (keyboard interactivity disabled)
- Single monitor only (all outputs receive widget)
- Configuration changes require restart
- Weather requires OpenWeatherMap API key

---

## Version History Summary

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | 2025-02-04 | Initial release with clock, weather, theming |

[Unreleased]: https://github.com/your-username/cosmic-desktop-widget/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-username/cosmic-desktop-widget/releases/tag/v0.1.0
