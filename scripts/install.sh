#!/bin/bash
# Install cosmic-desktop-widget and its configuration tool

set -e

# Build release binaries
cargo build --release

# Install binaries
sudo install -Dm755 target/release/cosmic-desktop-widget /usr/local/bin/
sudo install -Dm755 target/release/cosmic-desktop-widget-config /usr/local/bin/

# Install desktop entry
sudo install -Dm644 data/com.github.olafkfreund.CosmicDesktopWidget.Settings.desktop \
    /usr/share/applications/

echo "Installation complete!"
echo "Run 'cosmic-desktop-widget' to start the widget"
echo "Run 'cosmic-desktop-widget-config' to configure"
