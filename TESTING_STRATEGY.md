# Testing Strategy

## Overview

This document defines the testing strategy for the COSMIC Desktop Widget project, including unit tests, integration tests, and manual testing procedures.

---

## Testing Philosophy

### Goals

1. **Confidence** - Tests give us confidence the code works
2. **Documentation** - Tests document expected behavior
3. **Regression Prevention** - Tests catch regressions
4. **Design Feedback** - Hard-to-test code is usually bad code

### Principles

- **Test behavior, not implementation** - Tests should survive refactoring
- **Fast tests** - Unit tests should run in < 1 second total
- **Deterministic** - No flaky tests
- **Isolated** - Tests don't depend on each other
- **Readable** - Tests are documentation

---

## Test Pyramid

```
         ┌─────────────┐
         │   Manual    │  ← 5% - Exploratory testing
         │   Testing   │
         ├─────────────┤
         │ Integration │  ← 20% - Full workflow tests
         │    Tests    │
         ├─────────────┤
         │    Unit     │  ← 75% - Fast, isolated tests
         │    Tests    │
         └─────────────┘
```

### Distribution

- **75% Unit Tests** - Test individual functions/modules
- **20% Integration Tests** - Test component interactions
- **5% Manual Tests** - Visual verification, exploratory testing

---

## Unit Tests

### What to Test

**Core Logic**:
- ✅ Configuration parsing/validation
- ✅ Widget update logic
- ✅ Time formatting
- ✅ Data transformations
- ✅ Error handling

**Not to Test**:
- ❌ Third-party library internals (tiny-skia, smithay)
- ❌ Wayland protocol details
- ❌ Visual rendering output
- ❌ Trivial getters/setters

### Structure

```rust
// src/widget/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_formatting_24h() {
        // Arrange
        let clock = ClockWidget::new();
        
        // Act
        let time_str = clock.format_time_24h(14, 35, 22);
        
        // Assert
        assert_eq!(time_str, "14:35:22");
    }

    #[test]
    fn test_clock_formatting_12h() {
        let clock = ClockWidget::new();
        
        let time_str = clock.format_time_12h(14, 35, 22);
        
        assert_eq!(time_str, "2:35:22 PM");
    }

    #[test]
    fn test_weather_display_with_data() {
        let mut weather = WeatherWidget::new("London", "key");
        weather.data = Some(WeatherData {
            temperature: 22.0,
            condition: "Sunny".to_string(),
            humidity: 65,
            wind_speed: 5.2,
        });
        
        let display = weather.display_string();
        
        assert_eq!(display, Some("22°C Sunny | 65% humidity".to_string()));
    }

    #[test]
    fn test_weather_display_without_data() {
        let weather = WeatherWidget::new("London", "key");
        
        let display = weather.display_string();
        
        assert_eq!(display, None);
    }
}
```

### Testing Error Conditions

```rust
#[test]
fn test_config_invalid_position() {
    let toml = r#"
        width = 400
        height = 150
        position = "invalid"
    "#;
    
    let result: Result<Config, _> = toml::from_str(toml);
    
    assert!(result.is_err());
}

#[test]
fn test_config_missing_required_field() {
    let toml = r#"
        width = 400
        # missing height
    "#;
    
    let result: Result<Config, _> = toml::from_str(toml);
    
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "width must be > 0")]
fn test_invalid_width_panics() {
    create_surface(0, 150);
}
```

### Testing with Mocks

```rust
// Mock for external API
struct MockWeatherApi {
    response: WeatherData,
}

impl WeatherApi for MockWeatherApi {
    async fn fetch(&self, _city: &str) -> Result<WeatherData> {
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn test_weather_fetch_success() {
    let mock = MockWeatherApi {
        response: WeatherData {
            temperature: 20.0,
            condition: "Rainy".to_string(),
            humidity: 80,
            wind_speed: 10.0,
        },
    };
    
    let mut widget = WeatherWidget::with_api(Box::new(mock));
    
    let result = widget.fetch_weather().await;
    
    assert!(result.is_ok());
    assert_eq!(widget.data.as_ref().unwrap().temperature, 20.0);
}
```

### Property-Based Testing (Advanced)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_time_formatting_never_panics(hour in 0u32..24, min in 0u32..60, sec in 0u32..60) {
        let clock = ClockWidget::new();
        let _ = clock.format_time_24h(hour, min, sec);
        // Should not panic for any valid time
    }
    
    #[test]
    fn test_config_serialization_roundtrip(
        width in 100u32..1000,
        height in 100u32..1000
    ) {
        let config = Config {
            width,
            height,
            ..Default::default()
        };
        
        let toml = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml).unwrap();
        
        assert_eq!(config.width, parsed.width);
        assert_eq!(config.height, parsed.height);
    }
}
```

---

## Integration Tests

### Structure

```
tests/
├── integration/
│   ├── mod.rs
│   ├── config_integration.rs
│   ├── widget_lifecycle.rs
│   └── wayland_integration.rs
└── common/
    └── mod.rs  # Shared test utilities
```

### Example: Configuration Integration

```rust
// tests/integration/config_integration.rs
use cosmic_desktop_widget::config::Config;
use tempfile::TempDir;

#[test]
fn test_config_full_lifecycle() {
    // Create temp directory for config
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    // Create default config
    let config = Config::default();
    config.save_to(&config_path).unwrap();
    
    // Verify file exists
    assert!(config_path.exists());
    
    // Load config
    let loaded = Config::load_from(&config_path).unwrap();
    
    // Verify values match
    assert_eq!(config.width, loaded.width);
    assert_eq!(config.height, loaded.height);
    
    // Modify and save
    let mut modified = loaded;
    modified.width = 500;
    modified.save_to(&config_path).unwrap();
    
    // Reload and verify
    let reloaded = Config::load_from(&config_path).unwrap();
    assert_eq!(reloaded.width, 500);
}
```

### Example: Widget Lifecycle

```rust
// tests/integration/widget_lifecycle.rs
#[test]
fn test_widget_update_cycle() {
    let mut clock = ClockWidget::new();
    let initial_time = clock.time_string();
    
    // Wait for second to pass
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Update widget
    clock.update();
    let updated_time = clock.time_string();
    
    // Time should have changed
    assert_ne!(initial_time, updated_time);
}

#[test]
fn test_multiple_widget_updates() {
    let mut widgets = vec![
        Box::new(ClockWidget::new()) as Box<dyn Widget>,
        Box::new(WeatherWidget::new("London", "key")) as Box<dyn Widget>,
    ];
    
    // Update all widgets
    for widget in &mut widgets {
        let result = widget.update();
        assert!(result.is_ok());
    }
    
    // All should have display strings
    for widget in &widgets {
        let display = widget.display_string();
        assert!(display.is_some());
    }
}
```

### Mocking Wayland (Advanced)

```rust
// tests/common/mod.rs
pub struct MockWaylandEnv {
    _server: wayland_server::Display,
    // ... mock server setup
}

impl MockWaylandEnv {
    pub fn new() -> Self {
        // Create mock Wayland server
        // Useful for testing without real compositor
        todo!()
    }
}

// tests/integration/wayland_integration.rs
#[test]
fn test_surface_creation() {
    let env = MockWaylandEnv::new();
    
    // Test creating layer surface
    // ...
}
```

---

## Manual Testing

### Test Scenarios

#### Scenario 1: Basic Functionality

**Steps**:
1. Start COSMIC Desktop
2. Create default config: `just create-config`
3. Run widget: `just run`
4. **Verify**: Widget appears in top-right corner
5. **Verify**: Clock shows current time
6. **Verify**: Weather shows data (if API key set)
7. **Verify**: Updates every second

**Expected**: Widget displays correctly, updates work

---

#### Scenario 2: Configuration Changes

**Steps**:
1. Edit config: Change position to "bottom-left"
2. Change size to 500x200
3. Restart widget
4. **Verify**: Widget moves to bottom-left
5. **Verify**: Widget is larger size

**Expected**: Configuration applies immediately

---

#### Scenario 3: Multiple Displays

**Steps**:
1. Connect second monitor
2. Run widget
3. **Verify**: Widget appears on all displays (if `output = None`)
4. Or appears on specific display (if configured)

**Expected**: Correct multi-display behavior

---

#### Scenario 4: Error Handling

**Steps**:
1. Set invalid API key in config
2. Run widget with `RUST_LOG=debug`
3. **Verify**: Weather shows error state or placeholder
4. **Verify**: Logs show clear error message
5. **Verify**: Widget continues to work (clock still updates)

**Expected**: Graceful degradation, clear errors

---

#### Scenario 5: Performance

**Steps**:
1. Run widget
2. Monitor with `htop` for 5 minutes
3. **Verify**: Memory usage stable (< 50 MB)
4. **Verify**: CPU usage low (< 1% average)
5. Trigger 100 updates rapidly
6. **Verify**: No performance degradation

**Expected**: Stable performance over time

---

#### Scenario 6: Compositor Compatibility

**Steps**:
1. Test on COSMIC Desktop ✅
2. Test on Sway ✅
3. Test on Hyprland (if available)
4. **Verify**: Widget appears correctly on each
5. **Verify**: Positioning works
6. **Verify**: No compositor crashes

**Expected**: Works on all Layer Shell compositors

---

### Visual Testing Checklist

- [ ] Widget renders correctly
- [ ] Text is readable
- [ ] Colors match theme/config
- [ ] Borders/shadows render properly
- [ ] No visual artifacts
- [ ] Transparency works (if configured)
- [ ] Updates are smooth (no flickering)
- [ ] Icons render correctly (if applicable)

---

## Test Data

### Mock Weather Data

```rust
pub fn mock_weather_sunny() -> WeatherData {
    WeatherData {
        temperature: 22.0,
        condition: "Sunny".to_string(),
        humidity: 65,
        wind_speed: 5.2,
    }
}

pub fn mock_weather_rainy() -> WeatherData {
    WeatherData {
        temperature: 15.0,
        condition: "Rainy".to_string(),
        humidity: 90,
        wind_speed: 15.5,
    }
}

pub fn mock_weather_extreme() -> WeatherData {
    WeatherData {
        temperature: -40.0,  // Extreme cold
        condition: "Blizzard".to_string(),
        humidity: 100,
        wind_speed: 50.0,
    }
}
```

### Test Configurations

```rust
pub fn test_config_minimal() -> Config {
    Config {
        width: 300,
        height: 100,
        position: "top-right".to_string(),
        ..Default::default()
    }
}

pub fn test_config_large() -> Config {
    Config {
        width: 800,
        height: 400,
        position: "center".to_string(),
        ..Default::default()
    }
}

pub fn test_config_all_features() -> Config {
    Config {
        width: 400,
        height: 150,
        position: "bottom-left".to_string(),
        show_clock: true,
        show_weather: true,
        clock_format: "12h".to_string(),
        temperature_unit: "fahrenheit".to_string(),
        ..Default::default()
    }
}
```

---

## Running Tests

### Basic Commands

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_clock_formatting

# Run tests in specific module
cargo test widget::tests

# Run integration tests only
cargo test --test '*'

# Run with just
just test
```

### With Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin --out Html

# View coverage report
firefox tarpaulin-report.html
```

### Watch Mode

```bash
# Install cargo-watch
cargo install cargo-watch

# Run tests on file change
cargo watch -x test

# Run specific test on change
cargo watch -x 'test test_clock_formatting'
```

---

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v2
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Check formatting
        run: cargo fmt -- --check
  
  coverage:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v2
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      
      - name: Generate coverage
        run: cargo tarpaulin --out Xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v2
```

---

## Test Maintenance

### When to Update Tests

- **Feature added** → Add tests for new feature
- **Bug fixed** → Add regression test
- **API changed** → Update affected tests
- **Refactored** → Tests should still pass (behavior unchanged)

### Red Flags

- **Flaky tests** → Fix immediately, don't ignore
- **Slow tests** → Profile and optimize
- **Unclear tests** → Add comments, improve naming
- **Brittle tests** → Test behavior, not implementation

### Test Smells

```rust
// ❌ BAD: Testing implementation details
#[test]
fn test_internal_field() {
    let widget = Widget::new();
    assert_eq!(widget.internal_counter, 0);  // Implementation detail!
}

// ✅ GOOD: Testing behavior
#[test]
fn test_widget_initial_state() {
    let widget = Widget::new();
    assert!(!widget.has_updates());  // Public behavior
}

// ❌ BAD: Magic numbers
#[test]
fn test_calculation() {
    assert_eq!(calculate(5), 10);  // Why 10?
}

// ✅ GOOD: Clear intent
#[test]
fn test_calculation_doubles_input() {
    let input = 5;
    let expected = input * 2;
    assert_eq!(calculate(input), expected);
}
```

---

## Performance Testing

### Benchmark Setup

```rust
// benches/render_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cosmic_desktop_widget::render::Renderer;

fn render_benchmark(c: &mut Criterion) {
    let mut renderer = Renderer::new();
    let mut canvas = vec![0u8; 400 * 150 * 4];
    
    c.bench_function("render_widget", |b| {
        b.iter(|| {
            renderer.render(
                black_box(&mut canvas),
                black_box(400),
                black_box(150),
            )
        })
    });
}

criterion_group!(benches, render_benchmark);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run benchmarks
cargo bench

# Compare with baseline
cargo bench -- --save-baseline my-baseline
# ... make changes ...
cargo bench -- --baseline my-baseline
```

---

## Test Documentation

### Test Naming Convention

```rust
// Pattern: test_<component>_<scenario>_<expected_result>

#[test]
fn test_clock_format_24h_returns_colon_separated() { }

#[test]
fn test_weather_fetch_invalid_key_returns_error() { }

#[test]
fn test_config_missing_field_uses_default() { }
```

### Documenting Test Intent

```rust
/// Test that clock widget correctly formats time in 24-hour format.
/// 
/// This ensures that times like 14:35:22 are displayed correctly
/// rather than converting to 12-hour format.
#[test]
fn test_clock_formatting_24h() {
    // Given a clock widget configured for 24-hour format
    let clock = ClockWidget::with_format(ClockFormat::Hour24);
    
    // When formatting the time 14:35:22
    let result = clock.format_time(14, 35, 22);
    
    // Then the result should be "14:35:22"
    assert_eq!(result, "14:35:22");
}
```

---

## Resources

- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [proptest](https://github.com/proptest-rs/proptest)
- [criterion.rs](https://github.com/bheisler/criterion.rs)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)

---

**Target Coverage**: 70%  
**Target Test Count**: 100+ tests  
**Target Speed**: < 1 second for all unit tests

**Last Updated**: 2025-01-13
