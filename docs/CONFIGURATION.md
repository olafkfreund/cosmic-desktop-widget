# Configuration Reference

This document provides a complete reference for all configuration options available in COSMIC Desktop Widget.

## Configuration File Location

The configuration file is located at:

```
~/.config/cosmic-desktop-widget/config.toml
```

If this file does not exist, a default configuration is created on first run.

## Configuration Schema

### Widget Dimensions

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `width` | integer | `400` | Widget width in pixels (1-10000) |
| `height` | integer | `150` | Widget height in pixels (1-10000) |

**Example:**
```toml
width = 500
height = 200
```

**Validation:**
- Width and height must be positive (greater than 0)
- Width and height cannot exceed 10000 pixels
- Invalid values will cause the widget to use defaults

### Position

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `position` | string | `"top-right"` | Screen corner for widget placement |

**Valid values:**
- `"top-left"` - Upper left corner of screen
- `"top-right"` - Upper right corner of screen (default)
- `"bottom-left"` - Lower left corner of screen
- `"bottom-right"` - Lower right corner of screen
- `"center"` - Center of screen

**Example:**
```toml
position = "bottom-right"
```

### Margins

Margins control the distance from the screen edge to the widget.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `margin.top` | integer | `20` | Pixels from top edge |
| `margin.right` | integer | `20` | Pixels from right edge |
| `margin.bottom` | integer | `0` | Pixels from bottom edge |
| `margin.left` | integer | `0` | Pixels from left edge |

**Example:**
```toml
[margin]
top = 50
right = 50
bottom = 20
left = 20
```

**Notes:**
- Negative margins are allowed and may push the widget off-screen
- Margins are applied based on the anchor position (e.g., `top` margin only affects top-anchored positions)

### Clock Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_clock` | boolean | `true` | Enable/disable clock widget |
| `clock_format` | string | `"24h"` | Time format (`"24h"` or `"12h"`) |
| `show_seconds` | boolean | `true` | Display seconds in time |
| `show_date` | boolean | `false` | Display date alongside time |

**Example:**
```toml
show_clock = true
clock_format = "12h"
show_seconds = false
show_date = true
```

**Time Format Examples:**
- `24h` with seconds: `14:35:22`
- `24h` without seconds: `14:35`
- `12h` with seconds: `02:35:22 PM`
- `12h` without seconds: `02:35 PM`

**Date Format:**
When `show_date = true`, the date is displayed as: `Monday, January 15, 2025`

### Weather Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_weather` | boolean | `true` | Enable/disable weather widget |
| `weather_city` | string | `"London"` | City name for weather data |
| `weather_api_key` | string | `""` | OpenWeatherMap API key |
| `temperature_unit` | string | `"celsius"` | Temperature unit (`"celsius"` or `"fahrenheit"`) |
| `update_interval` | integer | `600` | Weather update interval in seconds |

**Example:**
```toml
show_weather = true
weather_city = "New York"
weather_api_key = "your-api-key-here"
temperature_unit = "fahrenheit"
update_interval = 900  # 15 minutes
```

**Getting an API Key:**
1. Visit [OpenWeatherMap](https://openweathermap.org/api)
2. Create a free account
3. Navigate to "API keys" in your account
4. Generate a new API key
5. Copy the key to your config file

**City Name Notes:**
- Use English city names for best results
- For cities with common names, you can specify country code: `"London,UK"` or `"Paris,FR"`
- The API uses fuzzy matching, so exact spelling is not required

**Update Interval:**
- Minimum recommended: 60 seconds (API rate limiting)
- Default: 600 seconds (10 minutes)
- Very short intervals (< 60s) will trigger a warning about API rate limits

### Theme Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"cosmic_dark"` | Theme name or `"custom"` |
| `custom_theme` | object | `null` | Custom theme definition (when `theme = "custom"`) |

**Built-in Themes:**

1. **cosmic_dark** (default)
   - Dark background with COSMIC blue accent
   - Semi-transparent (90% opacity)
   - White text on dark background

2. **light**
   - Light/white background
   - High opacity (95%)
   - Dark text on light background

3. **transparent_dark**
   - Very transparent dark background (50% opacity)
   - Larger corner radius
   - Good for wallpaper visibility

**Example (built-in theme):**
```toml
theme = "light"
```

**Example (custom theme):**
```toml
theme = "custom"

[custom_theme]
opacity = 0.85
border_width = 1.5
corner_radius = 12.0

[custom_theme.background]
r = 20
g = 20
b = 40
a = 220

[custom_theme.border]
r = 80
g = 80
b = 120
a = 255

[custom_theme.text_primary]
r = 255
g = 255
b = 255
a = 255

[custom_theme.text_secondary]
r = 200
g = 200
b = 220
a = 255

[custom_theme.accent]
r = 100
g = 150
b = 255
a = 255
```

**Custom Theme Properties:**

| Property | Type | Description |
|----------|------|-------------|
| `opacity` | float | Overall background opacity (0.0-1.0) |
| `border_width` | float | Border line width in pixels |
| `corner_radius` | float | Rounded corner radius in pixels |
| `background` | Color | Widget background color |
| `border` | Color | Widget border color |
| `text_primary` | Color | Main text color (clock, weather) |
| `text_secondary` | Color | Secondary text color (labels) |
| `accent` | Color | Accent color (decorative elements) |

**Color Format:**
```toml
[custom_theme.background]
r = 255    # Red (0-255)
g = 255    # Green (0-255)
b = 255    # Blue (0-255)
a = 255    # Alpha/opacity (0-255)
```

### Layout Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `padding` | float | `20.0` | Internal padding in pixels |
| `spacing` | float | `10.0` | Space between widgets in pixels |

**Example:**
```toml
padding = 15.0
spacing = 8.0
```

**Notes:**
- Padding is applied around the inside edge of the widget container
- Spacing is the gap between the clock and weather widgets
- Larger padding reduces the content area

## Complete Example Configuration

```toml
# Widget dimensions and position
width = 450
height = 160
position = "top-right"

# Screen edge margins
[margin]
top = 30
right = 30
bottom = 0
left = 0

# Clock configuration
show_clock = true
clock_format = "24h"
show_seconds = true
show_date = false

# Weather configuration
show_weather = true
weather_city = "Seattle"
weather_api_key = "your-openweathermap-api-key"
temperature_unit = "fahrenheit"
update_interval = 600

# Visual styling
theme = "cosmic_dark"
padding = 20.0
spacing = 10.0
```

## Configuration Validation

The widget validates configuration on load and will:

1. **Use defaults** if the config file is missing or corrupted
2. **Log warnings** for values that are valid but potentially problematic:
   - Update intervals less than 60 seconds
   - Very large or small dimensions
3. **Fall back to defaults** for invalid values:
   - Invalid position strings
   - Invalid clock format
   - Invalid temperature unit
   - Zero or negative dimensions

## Environment Variables

The widget respects the following environment variables:

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging level (`error`, `warn`, `info`, `debug`, `trace`) |
| `WAYLAND_DISPLAY` | Wayland display to connect to (usually `wayland-0`) |
| `XDG_CONFIG_HOME` | Base directory for config files (defaults to `~/.config`) |

**Example:**
```bash
RUST_LOG=debug cosmic-desktop-widget
```

## Reloading Configuration

Currently, configuration changes require restarting the widget:

```bash
# If running as systemd service
systemctl --user restart cosmic-desktop-widget

# If running manually
# Stop with Ctrl+C, then restart
cosmic-desktop-widget
```

## Troubleshooting Configuration Issues

### Config Not Loading

1. Check file exists: `ls -la ~/.config/cosmic-desktop-widget/config.toml`
2. Check file permissions: should be readable by your user
3. Validate TOML syntax: use a TOML validator or check logs for parse errors
4. Run with debug logging: `RUST_LOG=debug cosmic-desktop-widget`

### Invalid Values Being Ignored

The widget logs warnings when it falls back to defaults:

```bash
RUST_LOG=warn cosmic-desktop-widget 2>&1 | grep -i config
```

### Finding the Config Path

The configuration path follows XDG Base Directory Specification:

```bash
# Default path
~/.config/cosmic-desktop-widget/config.toml

# If XDG_CONFIG_HOME is set
$XDG_CONFIG_HOME/cosmic-desktop-widget/config.toml
```

## Default Configuration

If you need to reset to defaults, delete the config file:

```bash
rm ~/.config/cosmic-desktop-widget/config.toml
cosmic-desktop-widget  # Will create new default config
```

Or create a minimal config:

```toml
# Minimal config - all values use defaults
width = 400
height = 150
position = "top-right"

[margin]
top = 20
right = 20
bottom = 0
left = 0
```
