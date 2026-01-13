# Justfile for COSMIC Desktop Widget

# Default recipe
default:
    @just --list

# Build the project
build:
    cargo build --release

# Run the widget
run:
    RUST_LOG=info cargo run --release

# Run with debug logging
run-debug:
    RUST_LOG=debug cargo run

# Run with trace logging
run-trace:
    RUST_LOG=trace cargo run

# Run tests
test:
    cargo test

# Run clippy
check:
    cargo clippy --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run all checks
check-all: fmt-check check test

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
doc:
    cargo doc --no-deps --open

# Install to system (requires sudo)
install:
    cargo build --release
    sudo install -Dm755 target/release/cosmic-desktop-widget /usr/local/bin/cosmic-desktop-widget

# Uninstall from system
uninstall:
    sudo rm -f /usr/local/bin/cosmic-desktop-widget

# Check if running on Wayland
check-wayland:
    #!/usr/bin/env bash
    if [ -z "$WAYLAND_DISPLAY" ]; then
        echo "❌ Not running on Wayland!"
        echo "   WAYLAND_DISPLAY is not set"
        exit 1
    else
        echo "✅ Running on Wayland: $WAYLAND_DISPLAY"
    fi

# Check for Layer Shell support
check-layer-shell:
    #!/usr/bin/env bash
    if command -v weston-info &> /dev/null; then
        if weston-info | grep -q "zwlr_layer_shell"; then
            echo "✅ Layer Shell is supported"
        else
            echo "❌ Layer Shell is NOT supported"
            echo "   Your compositor doesn't support zwlr_layer_shell_v1"
        fi
    else
        echo "⚠️  weston-info not found, cannot check Layer Shell support"
        echo "   Install wayland-utils to check"
    fi

# Run system checks
check-system: check-wayland check-layer-shell

# Watch for changes and rebuild
watch:
    cargo watch -x build

# Benchmark
bench:
    cargo bench

# Generate Cargo.lock (for NixOS)
lock:
    cargo generate-lockfile

# Update dependencies
update:
    cargo update

# Create example config
create-config:
    #!/usr/bin/env bash
    CONFIG_DIR="$HOME/.config/cosmic-desktop-widget"
    CONFIG_FILE="$CONFIG_DIR/config.toml"
    
    mkdir -p "$CONFIG_DIR"
    
    if [ -f "$CONFIG_FILE" ]; then
        echo "Config already exists at: $CONFIG_FILE"
    else
        cat > "$CONFIG_FILE" << 'EOF'
# COSMIC Desktop Widget Configuration

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
weather_api_key = ""  # Get from https://openweathermap.org/api

# Update interval in seconds (600 = 10 minutes)
update_interval = 600

# Display options
show_clock = true
show_weather = true

# Clock format: "12h" or "24h"
clock_format = "24h"

# Temperature unit: "celsius" or "fahrenheit"
temperature_unit = "celsius"
EOF
        echo "✅ Created config at: $CONFIG_FILE"
        echo "   Edit it to customize your widget!"
    fi

# Show current config
show-config:
    @cat ~/.config/cosmic-desktop-widget/config.toml 2>/dev/null || echo "No config file found. Run 'just create-config'"
