# Widget Development Skill

## Overview

Guide for developing widgets for the COSMIC Desktop Widget system. Covers widget structure, lifecycle, rendering, and best practices.

---

## Widget Architecture

### Widget Trait (Future)

```rust
/// Core trait for all widgets
pub trait Widget {
    /// Update widget state
    fn update(&mut self) -> Result<()>;
    
    /// Get displayable content
    fn render(&self, renderer: &mut Renderer) -> Result<()>;
    
    /// Widget size requirements
    fn size(&self) -> (u32, u32);
    
    /// Widget needs redraw
    fn needs_redraw(&self) -> bool;
}
```

### Current Pattern (v0.1)

```rust
pub struct MyWidget {
    // Widget data
    data: String,
    
    // Update tracking
    last_update: Instant,
    update_interval: Duration,
    
    // State
    needs_redraw: bool,
}

impl MyWidget {
    pub fn new() -> Self {
        Self {
            data: String::new(),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(60),
            needs_redraw: true,
        }
    }
    
    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.update_interval {
            self.data = self.fetch_data();
            self.last_update = Instant::now();
            self.needs_redraw = true;
        }
    }
    
    pub fn display_string(&self) -> Option<String> {
        if self.data.is_empty() {
            None
        } else {
            Some(self.data.clone())
        }
    }
}
```

---

## Widget Types

### 1. Clock Widget

**Purpose**: Display current time and date

**Update Strategy**: Every second

**Data Source**: System clock (chrono::Local)

```rust
pub struct ClockWidget {
    current_time: String,
    last_update: Instant,
    format: ClockFormat,
}

impl ClockWidget {
    pub fn new() -> Self {
        Self {
            current_time: Self::format_now(),
            last_update: Instant::now(),
            format: ClockFormat::Hour24,
        }
    }
    
    pub fn update(&mut self) {
        if self.last_update.elapsed() >= Duration::from_secs(1) {
            self.current_time = Self::format_now();
            self.last_update = Instant::now();
        }
    }
    
    pub fn time_string(&self) -> &str {
        &self.current_time
    }
    
    fn format_now() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }
}
```

**Configuration**:
```toml
[clock]
enabled = true
format = "24h"  # or "12h"
show_seconds = true
show_date = false
```

---

### 2. Weather Widget

**Purpose**: Display current weather conditions

**Update Strategy**: Every 10 minutes (API rate limits)

**Data Source**: OpenWeatherMap API

```rust
pub struct WeatherWidget {
    city: String,
    api_key: String,
    data: Option<WeatherData>,
    last_update: Instant,
    update_interval: Duration,
}

#[derive(Clone, Debug)]
pub struct WeatherData {
    pub temperature: f32,
    pub condition: String,
    pub humidity: u32,
    pub wind_speed: f32,
}

impl WeatherWidget {
    pub fn new(city: &str, api_key: &str) -> Self {
        Self {
            city: city.to_string(),
            api_key: api_key.to_string(),
            data: None,
            last_update: Instant::now() - Duration::from_secs(1000), // Force first update
            update_interval: Duration::from_secs(600), // 10 minutes
        }
    }
    
    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.update_interval {
            // Spawn async task to fetch weather
            // Update self.data when complete
            self.last_update = Instant::now();
        }
    }
    
    pub fn display_string(&self) -> Option<String> {
        self.data.as_ref().map(|data| {
            format!(
                "{}°C {} | {}% humidity",
                data.temperature.round(),
                data.condition,
                data.humidity
            )
        })
    }
}
```

**API Integration**:
```rust
pub async fn fetch_weather(&self) -> Result<WeatherData> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
        self.city, self.api_key
    );
    
    let response = reqwest::get(&url).await?;
    let json: serde_json::Value = response.json().await?;
    
    Ok(WeatherData {
        temperature: json["main"]["temp"].as_f64().unwrap_or(0.0) as f32,
        condition: json["weather"][0]["main"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        humidity: json["main"]["humidity"].as_u64().unwrap_or(0) as u32,
        wind_speed: json["wind"]["speed"].as_f64().unwrap_or(0.0) as f32,
    })
}
```

---

### 3. System Monitor Widget (Example)

**Purpose**: Display CPU/RAM usage

**Update Strategy**: Every 2 seconds

**Data Source**: `/proc` filesystem or `sysinfo` crate

```rust
use sysinfo::{System, SystemExt, CpuExt};

pub struct SystemMonitorWidget {
    system: System,
    cpu_usage: f32,
    memory_usage: f32,
    last_update: Instant,
}

impl SystemMonitorWidget {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            last_update: Instant::now(),
        }
    }
    
    pub fn update(&mut self) {
        if self.last_update.elapsed() >= Duration::from_secs(2) {
            self.system.refresh_cpu();
            self.system.refresh_memory();
            
            self.cpu_usage = self.system.global_cpu_info().cpu_usage();
            self.memory_usage = (self.system.used_memory() as f32 
                / self.system.total_memory() as f32) * 100.0;
            
            self.last_update = Instant::now();
        }
    }
    
    pub fn display_string(&self) -> String {
        format!(
            "CPU: {:.1}% | RAM: {:.1}%",
            self.cpu_usage,
            self.memory_usage
        )
    }
}
```

---

### 4. Calendar Widget (Example)

**Purpose**: Display current date and upcoming events

**Update Strategy**: Every minute (at minute change)

**Data Source**: System time + calendar file/API

```rust
pub struct CalendarWidget {
    current_date: String,
    events: Vec<Event>,
    last_update: Instant,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub title: String,
    pub time: DateTime<Local>,
}

impl CalendarWidget {
    pub fn new() -> Self {
        Self {
            current_date: Local::now().format("%A, %B %d").to_string(),
            events: Vec::new(),
            last_update: Instant::now(),
        }
    }
    
    pub fn update(&mut self) {
        let now = Local::now();
        if now.second() == 0 || self.last_update.elapsed() >= Duration::from_secs(60) {
            self.current_date = now.format("%A, %B %d").to_string();
            self.events = self.fetch_today_events();
            self.last_update = Instant::now();
        }
    }
    
    pub fn display_string(&self) -> String {
        let mut display = self.current_date.clone();
        
        if !self.events.is_empty() {
            display.push_str("\n");
            for event in self.events.iter().take(3) {
                display.push_str(&format!(
                    "• {} - {}\n",
                    event.time.format("%H:%M"),
                    event.title
                ));
            }
        }
        
        display
    }
}
```

---

## Widget Rendering

### Basic Text Rendering

```rust
impl Renderer {
    pub fn render_widget_text(
        &self,
        pixmap: &mut PixmapMut,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [u8; 4],
    ) {
        // Use fontdue for actual text rendering
        // This is a simplified example
        
        let rect = Rect::from_xywh(x, y, font_size * 6.0, font_size)?;
        
        let mut paint = Paint::default();
        paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
        
        let path = PathBuilder::from_rect(rect);
        pixmap.fill_path(&path, &paint, ...);
    }
}
```

### Widget Layout

```rust
pub struct WidgetLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub padding: f32,
}

impl WidgetLayout {
    pub fn vertical_stack(widgets: &[Box<dyn Widget>], start_y: f32) -> Vec<WidgetLayout> {
        let mut layouts = Vec::new();
        let mut current_y = start_y;
        
        for widget in widgets {
            let (width, height) = widget.size();
            
            layouts.push(WidgetLayout {
                x: 10.0,
                y: current_y,
                width: width as f32,
                height: height as f32,
                padding: 5.0,
            });
            
            current_y += height as f32 + 10.0; // spacing
        }
        
        layouts
    }
}
```

---

## Widget Update Coordination

### Update Manager

```rust
pub struct WidgetManager {
    widgets: Vec<Box<dyn Widget>>,
    last_updates: HashMap<usize, Instant>,
}

impl WidgetManager {
    pub fn update_all(&mut self) -> bool {
        let mut any_updated = false;
        
        for (idx, widget) in self.widgets.iter_mut().enumerate() {
            if widget.needs_redraw() {
                widget.update().ok();
                self.last_updates.insert(idx, Instant::now());
                any_updated = true;
            }
        }
        
        any_updated
    }
    
    pub fn render_all(&self, renderer: &mut Renderer) -> Result<()> {
        for widget in &self.widgets {
            widget.render(renderer)?;
        }
        Ok(())
    }
}
```

---

## Best Practices

### 1. Efficient Updates

```rust
// ✅ GOOD: Only update when needed
impl Widget {
    pub fn update(&mut self) {
        if !self.should_update() {
            return;
        }
        
        self.fetch_data();
        self.needs_redraw = true;
    }
    
    fn should_update(&self) -> bool {
        self.last_update.elapsed() >= self.update_interval
    }
}

// ❌ BAD: Always fetching data
impl Widget {
    pub fn update(&mut self) {
        self.fetch_data();  // Every call!
    }
}
```

### 2. Error Handling

```rust
// ✅ GOOD: Graceful degradation
impl WeatherWidget {
    pub fn update(&mut self) {
        match self.fetch_weather() {
            Ok(data) => {
                self.data = Some(data);
                self.error = None;
            }
            Err(e) => {
                tracing::warn!("Weather fetch failed: {}", e);
                self.error = Some(e.to_string());
                // Keep old data if available
            }
        }
    }
    
    pub fn display_string(&self) -> Option<String> {
        if let Some(err) = &self.error {
            Some(format!("⚠️ Weather: {}", err))
        } else if let Some(data) = &self.data {
            Some(format!("{}°C {}", data.temperature, data.condition))
        } else {
            Some("Loading...".to_string())
        }
    }
}
```

### 3. Configuration

```rust
// ✅ GOOD: Configurable widgets
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherConfig {
    pub city: String,
    pub api_key: String,
    pub unit: TemperatureUnit,
    pub update_interval_secs: u64,
}

impl WeatherWidget {
    pub fn from_config(config: &WeatherConfig) -> Self {
        Self {
            city: config.city.clone(),
            api_key: config.api_key.clone(),
            unit: config.unit,
            update_interval: Duration::from_secs(config.update_interval_secs),
            // ...
        }
    }
}
```

### 4. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_widget_display_with_data() {
        let mut widget = WeatherWidget::new("London", "key");
        widget.data = Some(WeatherData {
            temperature: 20.0,
            condition: "Sunny".to_string(),
            humidity: 60,
            wind_speed: 5.0,
        });
        
        let display = widget.display_string();
        assert!(display.is_some());
        assert!(display.unwrap().contains("20°C"));
    }
    
    #[test]
    fn test_widget_handles_no_data() {
        let widget = WeatherWidget::new("London", "key");
        
        let display = widget.display_string();
        assert!(display.is_some());  // Should show "Loading..." or similar
    }
}
```

---

## Widget Lifecycle

```
┌──────────────────────┐
│  Widget Created      │
│  (new())             │
└──────┬───────────────┘
       │
       v
┌──────────────────────┐
│  Initial State       │
│  - No data           │
│  - needs_redraw=true │
└──────┬───────────────┘
       │
       v
┌──────────────────────┐
│  First Update        │
│  - Fetch data        │
│  - Set initial state │
└──────┬───────────────┘
       │
       v
┌──────────────────────┐
│  Render              │
│  - Draw to canvas    │
│  - needs_redraw=false│
└──────┬───────────────┘
       │
       v
   ┌───┴────────────────┐
   │                    │
   v                    v
┌──────────────┐  ┌──────────────┐
│ Periodic     │  │ External     │
│ Update       │  │ Event        │
│ (timer)      │  │ (config)     │
└──┬───────────┘  └──┬───────────┘
   │                 │
   └────────┬────────┘
            │
            v
      ┌─────────────┐
      │ Update Loop │ ← Repeat
      └─────────────┘
```

---

## Performance Optimization

### 1. Caching

```rust
pub struct CachedWidget {
    data: Option<Data>,
    rendered: Option<Vec<u8>>,  // Cached rendering
    data_dirty: bool,
}

impl CachedWidget {
    pub fn render(&mut self, canvas: &mut [u8]) {
        if !self.data_dirty && self.rendered.is_some() {
            // Use cached rendering
            canvas.copy_from_slice(self.rendered.as_ref().unwrap());
            return;
        }
        
        // Re-render
        self.do_render(canvas);
        self.rendered = Some(canvas.to_vec());
        self.data_dirty = false;
    }
}
```

### 2. Lazy Updates

```rust
pub struct LazyWidget {
    update_counter: u64,
    update_frequency: u64,  // Update every N calls
}

impl LazyWidget {
    pub fn update(&mut self) {
        self.update_counter += 1;
        
        if self.update_counter % self.update_frequency == 0 {
            self.do_expensive_update();
        }
    }
}
```

### 3. Async Data Fetching

```rust
pub struct AsyncWidget {
    data: Arc<Mutex<Option<Data>>>,
    fetcher_handle: Option<JoinHandle<()>>,
}

impl AsyncWidget {
    pub fn start_fetch(&mut self) {
        let data = Arc::clone(&self.data);
        
        self.fetcher_handle = Some(tokio::spawn(async move {
            match fetch_data().await {
                Ok(new_data) => {
                    *data.lock().unwrap() = Some(new_data);
                }
                Err(e) => {
                    tracing::error!("Fetch failed: {}", e);
                }
            }
        }));
    }
    
    pub fn display_string(&self) -> Option<String> {
        self.data.lock().unwrap()
            .as_ref()
            .map(|d| d.to_string())
    }
}
```

---

## Widget Configuration Schema

### Per-Widget Config

```toml
# ~/.config/cosmic-desktop-widget/config.toml

[clock]
enabled = true
format = "24h"
show_seconds = true
show_date = false

[weather]
enabled = true
city = "London"
api_key = "your-key-here"
unit = "celsius"
update_interval = 600  # seconds

[system_monitor]
enabled = false
show_cpu = true
show_memory = true
show_network = false
update_interval = 2
```

### Loading Widget Config

```rust
#[derive(Deserialize)]
pub struct WidgetConfigs {
    pub clock: Option<ClockConfig>,
    pub weather: Option<WeatherConfig>,
    pub system_monitor: Option<SystemMonitorConfig>,
}

impl WidgetConfigs {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }
    
    pub fn create_widgets(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = Vec::new();
        
        if let Some(clock_cfg) = &self.clock {
            if clock_cfg.enabled {
                widgets.push(Box::new(ClockWidget::from_config(clock_cfg)));
            }
        }
        
        if let Some(weather_cfg) = &self.weather {
            if weather_cfg.enabled {
                widgets.push(Box::new(WeatherWidget::from_config(weather_cfg)));
            }
        }
        
        widgets
    }
}
```

---

## Widget Template

```rust
// Template for creating new widgets

pub struct MyWidget {
    // Widget data
    data: Option<MyData>,
    
    // Configuration
    config: MyWidgetConfig,
    
    // Update tracking
    last_update: Instant,
    update_interval: Duration,
    
    // State
    needs_redraw: bool,
    error: Option<String>,
}

impl MyWidget {
    pub fn new(config: MyWidgetConfig) -> Self {
        Self {
            data: None,
            config,
            last_update: Instant::now() - Duration::from_secs(1000),
            update_interval: Duration::from_secs(config.update_interval),
            needs_redraw: true,
            error: None,
        }
    }
    
    pub fn update(&mut self) {
        // Check if update needed
        if self.last_update.elapsed() < self.update_interval {
            return;
        }
        
        // Fetch/compute data
        match self.fetch_data() {
            Ok(data) => {
                self.data = Some(data);
                self.error = None;
                self.needs_redraw = true;
            }
            Err(e) => {
                tracing::warn!("Widget update failed: {}", e);
                self.error = Some(e.to_string());
            }
        }
        
        self.last_update = Instant::now();
    }
    
    pub fn display_string(&self) -> Option<String> {
        if let Some(err) = &self.error {
            Some(format!("⚠️ {}", err))
        } else {
            self.data.as_ref().map(|d| d.to_string())
        }
    }
    
    fn fetch_data(&self) -> Result<MyData> {
        // Implement data fetching
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_widget_creation() {
        let config = MyWidgetConfig::default();
        let widget = MyWidget::new(config);
        assert!(widget.data.is_none());
    }
    
    #[test]
    fn test_widget_update() {
        // Test update logic
    }
}
```

---

## Resources

- [chrono docs](https://docs.rs/chrono/) - Time handling
- [reqwest docs](https://docs.rs/reqwest/) - HTTP requests
- [sysinfo docs](https://docs.rs/sysinfo/) - System information
- [tokio docs](https://docs.rs/tokio/) - Async runtime

---

**Last Updated**: 2025-01-13
