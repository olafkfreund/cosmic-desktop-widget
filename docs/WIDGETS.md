# Widget Development Guide

This guide explains how to create new widgets for COSMIC Desktop Widget.

## Available Widgets

| Widget | Type ID | Description | Status |
|--------|---------|-------------|--------|
| **Clock** | `clock` | Displays current time with configurable format | ✅ Stable |
| **Weather** | `weather` | Shows weather from OpenWeatherMap API | ✅ Stable |
| **System Monitor** | `system_monitor` | CPU, RAM, and disk usage | ✅ New |
| **Countdown** | `countdown` | Countdown timer to a target date | ✅ New |
| **Quotes** | `quotes` | Inspirational quotes display | ✅ New |

## Configuration

### New Configuration Format (v2)

Widgets are now configured as an ordered array in `~/.config/cosmic-desktop-widget/config.toml`:

```toml
# Panel settings
[panel]
width = 450
height = 180

# Widget position on screen (see "Widget Positioning" section below)
# Options: top-left, top-center, top-right,
#          center-left, center, center-right,
#          bottom-left, bottom-center, bottom-right
position = "top-right"

theme = "cosmic_dark"
padding = 20.0
spacing = 10.0

# Optional: Override background opacity (0.0 = fully transparent, 1.0 = fully opaque)
# background_opacity = 0.8

[panel.margin]
top = 10
right = 20
bottom = 0
left = 0

# Widgets - order determines display order!
[[widgets]]
type = "clock"
enabled = true

[widgets.config]
format = "24h"
show_seconds = true
show_date = false

[[widgets]]
type = "weather"
enabled = true

[widgets.config]
city = "London"
api_key = "your-openweathermap-api-key"
temperature_unit = "celsius"
update_interval = 600

[[widgets]]
type = "system_monitor"
enabled = true

[widgets.config]
show_cpu = true
show_memory = true
show_disk = false
update_interval = 2

[[widgets]]
type = "countdown"
enabled = true

[widgets.config]
label = "New Year"
target_date = "2026-01-01"
show_days = true
show_hours = true
show_minutes = true
show_seconds = false

[[widgets]]
type = "quotes"
enabled = true

[widgets.config]
rotation_interval = 60
random = true
# Optional: custom quotes file
# quotes_file = "~/.config/cosmic-desktop-widget/quotes.json"
```

### Widget Positioning

The `position` setting controls where the widget appears on your screen. All 9 positions are supported:

```
┌─────────────────────────────────────────────────┐
│  top-left     top-center      top-right         │
│                                                  │
│                                                  │
│  center-left     center       center-right      │
│                                                  │
│                                                  │
│  bottom-left  bottom-center   bottom-right      │
└─────────────────────────────────────────────────┘
```

#### Position Options

| Position | Description | Wayland Anchor |
|----------|-------------|----------------|
| `top-left` | Top-left corner | TOP + LEFT |
| `top-center` | Top edge, horizontally centered | TOP |
| `top-right` | Top-right corner (default) | TOP + RIGHT |
| `center-left` | Left edge, vertically centered | LEFT |
| `center` | Screen center | None (floating) |
| `center-right` | Right edge, vertically centered | RIGHT |
| `bottom-left` | Bottom-left corner | BOTTOM + LEFT |
| `bottom-center` | Bottom edge, horizontally centered | BOTTOM |
| `bottom-right` | Bottom-right corner | BOTTOM + RIGHT |

#### How Positioning Works

The widget uses the Wayland Layer Shell protocol to position itself:

- **Corner positions** (e.g., `top-left`): Widget is anchored to both edges
- **Edge positions** (e.g., `top-center`): Widget is anchored to one edge, centered on the other axis
- **Center position**: Widget floats in the center of the screen with no anchors

#### Margins

Margins control spacing from screen edges and are applied relative to the anchored edges:

```toml
[panel.margin]
top = 10      # Spacing from top edge (for top-* positions)
right = 20    # Spacing from right edge (for *-right positions)
bottom = 10   # Spacing from bottom edge (for bottom-* positions)
left = 20     # Spacing from left edge (for *-left positions)
```

**Note:** The widget automatically detects COSMIC panels and adjusts margins to avoid overlap.

#### Examples

**Top-right corner with margin:**
```toml
[panel]
position = "top-right"

[panel.margin]
top = 10
right = 20
```

**Bottom-center (panel-like):**
```toml
[panel]
position = "bottom-center"

[panel.margin]
bottom = 0  # Flush with bottom
```

**Floating center:**
```toml
[panel]
position = "center"
# Margins are ignored for center position
```

### Widget Configuration Reference

#### Clock Widget

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `format` | string | `"24h"` | Time format: `"12h"` or `"24h"` |
| `show_seconds` | bool | `true` | Display seconds |
| `show_date` | bool | `false` | Display date alongside time |

#### Weather Widget

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `city` | string | `"London"` | City name for weather data |
| `api_key` | string | `""` | OpenWeatherMap API key |
| `temperature_unit` | string | `"celsius"` | `"celsius"` or `"fahrenheit"` |
| `update_interval` | int | `600` | Update interval in seconds |

#### System Monitor Widget

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `show_cpu` | bool | `true` | Display CPU usage |
| `show_memory` | bool | `true` | Display RAM usage |
| `show_disk` | bool | `false` | Display disk usage |
| `update_interval` | int | `2` | Update interval in seconds |

#### Countdown Widget

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `label` | string | `"Countdown"` | Event name/label |
| `target_date` | string | `"2026-01-01"` | Target date (YYYY-MM-DD or YYYY-MM-DD HH:MM:SS) |
| `show_days` | bool | `true` | Display days remaining |
| `show_hours` | bool | `true` | Display hours remaining |
| `show_minutes` | bool | `true` | Display minutes remaining |
| `show_seconds` | bool | `false` | Display seconds remaining |

#### Quotes Widget

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `rotation_interval` | int | `60` | How often to change quotes (seconds) |
| `random` | bool | `true` | Random order vs sequential |
| `quotes_file` | string | - | Optional path to custom quotes JSON file |

Custom quotes file format:
```json
[
  {"text": "Your quote here", "author": "Author Name"},
  {"text": "Another quote without author"}
]
```

## Transparency Configuration

The widget supports sophisticated transparency with multiple theme variants and per-panel opacity overrides.

### Theme Variants

| Theme | Description | Opacity | Blur | Best For |
|-------|-------------|---------|------|----------|
| `cosmic_dark` | Default COSMIC dark theme | 90% | No | General use |
| `light` | Light background theme | 95% | No | Light backgrounds |
| `transparent_dark` | Very transparent, light text | 50% | No | See-through with dark bg |
| `transparent_light` | Very transparent, dark text | 50% | No | See-through with light bg |
| `glass` | Glass-like with blur hint | 70% | Yes | Modern blur-capable compositors |

### Configuring Transparency

#### Use a Transparent Theme

```toml
[panel]
theme = "transparent_dark"  # Or "transparent_light" or "glass"
```

#### Override Opacity for Any Theme

You can override the background opacity for any theme:

```toml
[panel]
theme = "cosmic_dark"
background_opacity = 0.6  # 60% opacity (0.0 = transparent, 1.0 = opaque)
```

#### Glass Theme with Compositor Blur

The `glass` theme enables a blur hint for Wayland compositors that support background blur:

```toml
[panel]
theme = "glass"
```

**Note:** Blur is compositor-dependent. Currently supported by:
- KWin (KDE Plasma)
- Mutter with extensions (GNOME)
- Future COSMIC compositor support planned

### Transparency Examples

#### Minimal transparency (95% opaque)
```toml
[panel]
theme = "cosmic_dark"
background_opacity = 0.95
```

#### Medium transparency for wallpaper visibility
```toml
[panel]
theme = "transparent_dark"  # Uses 50% opacity by default
```

#### Highly transparent with custom override
```toml
[panel]
theme = "transparent_light"
background_opacity = 0.3  # Override to 30% opacity
```

#### Glass effect (requires compositor support)
```toml
[panel]
theme = "glass"  # 70% opacity with blur hint
```

### Text Readability

All transparent themes ensure text remains readable:
- **transparent_dark**: White text on dark transparent background
- **transparent_light**: Black text on light transparent background
- **glass**: White text with moderate transparency and blur

The renderer automatically applies proper alpha blending in ARGB8888 format for correct transparency.

## Widget Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      WidgetRegistry                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              DynWidgetFactory Traits                 │    │
│  │  ClockFactory │ WeatherFactory │ SystemMonFactory   │    │
│  └───────────────────────┬─────────────────────────────┘    │
│                          │                                   │
│                          ▼                                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │            Vec<Box<dyn Widget>>                      │    │
│  │  Clock │ Weather │ SystemMonitor │ Countdown │ ...  │    │
│  └───────────────────────┬─────────────────────────────┘    │
│                          │                                   │
│                          ▼                                   │
│                    ┌──────────┐                             │
│                    │ Renderer │                             │
│                    └──────────┘                             │
└─────────────────────────────────────────────────────────────┘
```

## Creating a New Widget

### Step 1: Create Widget Struct

Create a new file `src/widget/your_widget.rs`:

```rust
use super::traits::{Widget, WidgetContent, WidgetInfo, FontSize};
use super::registry::DynWidgetFactory;
use std::time::{Duration, Instant};

pub struct YourWidget {
    data: String,
    last_update: Instant,
    config_option: String,
}

impl YourWidget {
    pub fn new(config_option: &str) -> Self {
        Self {
            data: String::new(),
            last_update: Instant::now(),
            config_option: config_option.to_string(),
        }
    }

    pub fn display_string(&self) -> String {
        self.data.clone()
    }
}
```

### Step 2: Implement the Widget Trait

```rust
impl Widget for YourWidget {
    fn info(&self) -> WidgetInfo {
        WidgetInfo {
            id: "your_widget",
            name: "Your Widget",
            preferred_height: 40.0,
            min_height: 30.0,
            expand: false,
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() >= self.update_interval() {
            self.data = "Updated data".to_string();
            self.last_update = Instant::now();
        }
    }

    fn content(&self) -> WidgetContent {
        WidgetContent::Text {
            text: self.display_string(),
            size: FontSize::Medium,
        }
    }

    fn update_interval(&self) -> Duration {
        Duration::from_secs(60)
    }
}
```

### Step 3: Create Widget Factory

```rust
pub struct YourWidgetFactory;

impl DynWidgetFactory for YourWidgetFactory {
    fn widget_type(&self) -> &'static str {
        "your_widget"
    }

    fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
        let option = config
            .get("option")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        Ok(Box::new(YourWidget::new(option)))
    }

    fn default_config(&self) -> toml::Table {
        let mut config = toml::Table::new();
        config.insert("option".to_string(), toml::Value::String("default".to_string()));
        config
    }

    fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
        // Validate configuration options
        Ok(())
    }
}
```

### Step 4: Register in Widget Module

Edit `src/widget/mod.rs`:

```rust
pub mod your_widget;
pub use your_widget::{YourWidget, YourWidgetFactory};
```

### Step 5: Register Factory in Registry

Edit `src/widget/registry.rs`:

```rust
use super::your_widget::YourWidgetFactory;

impl WidgetRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(ClockWidgetFactory);
        registry.register(WeatherWidgetFactory);
        registry.register(YourWidgetFactory);  // Add here
        registry
    }
}
```

## Widget Content Types

```rust
pub enum WidgetContent {
    /// Single line of text
    Text { text: String, size: FontSize },

    /// Multiple lines
    MultiLine { lines: Vec<(String, FontSize)> },

    /// Text with icon (future)
    IconText { icon: String, text: String, size: FontSize },

    /// Progress bar
    Progress { value: f32, label: Option<String> },

    /// Nothing to render
    Empty,
}
```

## Font Sizes

```rust
pub enum FontSize {
    Large,       // Primary content (clock)
    Medium,      // Secondary content (weather)
    Small,       // Labels, status text
    Custom(f32), // Specific pixel size
}
```

## Best Practices

### 1. Efficient Updates

Only perform updates when the interval has elapsed:

```rust
fn update(&mut self) {
    if self.last_update.elapsed() < self.update_interval() {
        return;
    }
    // Perform update...
    self.last_update = Instant::now();
}
```

### 2. Error Handling

Use Result types and provide meaningful errors:

```rust
fn create(&self, config: &toml::Table) -> anyhow::Result<Box<dyn Widget>> {
    let value = config.get("value")
        .and_then(|v| v.as_str())
        .context("'value' is required")?;

    if value.is_empty() {
        anyhow::bail!("'value' cannot be empty");
    }

    Ok(Box::new(YourWidget::new(value)))
}
```

### 3. Configuration Validation

Validate in the factory's `validate_config` method:

```rust
fn validate_config(&self, config: &toml::Table) -> anyhow::Result<()> {
    if let Some(interval) = config.get("update_interval") {
        let val = interval.as_integer()
            .context("'update_interval' must be an integer")?;
        if val < 1 {
            anyhow::bail!("'update_interval' must be at least 1");
        }
    }
    Ok(())
}
```

### 4. Resource Cleanup

Implement `Drop` if your widget holds resources:

```rust
impl Drop for YourWidget {
    fn drop(&mut self) {
        // Cleanup resources
    }
}
```

## Testing Your Widget

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let widget = YourWidget::new("option");
        assert_eq!(widget.info().id, "your_widget");
    }

    #[test]
    fn test_factory_creation() {
        let factory = YourWidgetFactory;
        let config = factory.default_config();
        let widget = factory.create(&config).unwrap();
        assert_eq!(widget.info().id, "your_widget");
    }

    #[test]
    fn test_display_string() {
        let widget = YourWidget::new("option");
        let display = widget.display_string();
        // Assert on display string content
    }
}
```

## Future Widget Ideas

| Widget | Description | Data Source | Complexity |
|--------|-------------|-------------|------------|
| **Stocks** | Stock prices | Yahoo Finance API | Medium |
| **Crypto** | Cryptocurrency prices | CoinGecko API | Low |
| **Pomodoro** | Focus timer | Internal state | Low |
| **Now Playing** | Current media | MPRIS D-Bus | Medium |
| **Calendar** | Upcoming events | ICS files | Medium |
| **Battery** | Power status | `/sys/class/power_supply` | Low |
| **News** | Headlines | RSS/NewsAPI | Medium |

## Migration from Old Config

The widget system automatically migrates old configuration files:

**Old format:**
```toml
width = 400
show_clock = true
show_weather = true
weather_city = "London"
```

**New format (generated automatically):**
```toml
[panel]
width = 400

[[widgets]]
type = "clock"
enabled = true

[[widgets]]
type = "weather"
enabled = true
[widgets.config]
city = "London"
```

## Need Help?

- Check existing widgets in `src/widget/` for examples
- See the Widget trait in `src/widget/traits.rs`
- See the WidgetRegistry in `src/widget/registry.rs`
- Open an issue on GitHub for questions
