#!/usr/bin/env bash
# Test script for config hot-reload functionality

set -e

CONFIG_DIR="${HOME}/.config/cosmic-desktop-widget"
CONFIG_FILE="${CONFIG_DIR}/config.toml"

echo "=== Config Hot-Reload Test ==="
echo ""
echo "This script will:"
echo "1. Backup your current config"
echo "2. Modify the config file"
echo "3. Wait for the widget to detect the change"
echo ""
echo "Make sure the widget is running in another terminal with:"
echo "  RUST_LOG=info cargo run"
echo ""
read -p "Press Enter to continue..."

# Backup config
if [ -f "${CONFIG_FILE}" ]; then
    cp "${CONFIG_FILE}" "${CONFIG_FILE}.backup"
    echo "✓ Backed up config to ${CONFIG_FILE}.backup"
else
    echo "⚠ No config file found at ${CONFIG_FILE}"
    echo "  The widget will create a default config on first run"
    exit 1
fi

echo ""
echo "Current panel dimensions:"
grep -A 1 "\[panel\]" "${CONFIG_FILE}" | grep -E "width|height"

echo ""
echo "Modifying config to change panel size..."
sleep 2

# Modify the config
sed -i 's/^width = .*/width = 500/' "${CONFIG_FILE}"
sed -i 's/^height = .*/height = 200/' "${CONFIG_FILE}"

echo "✓ Updated config:"
grep -A 1 "\[panel\]" "${CONFIG_FILE}" | grep -E "width|height"

echo ""
echo "Watch the widget terminal for the reload message:"
echo '  "Config reload triggered by file change"'
echo ""
echo "Press Enter to restore original config..."
read

# Restore backup
mv "${CONFIG_FILE}.backup" "${CONFIG_FILE}"
echo "✓ Restored original config"

echo ""
echo "Test complete!"
echo "Check the widget terminal to verify it reloaded the config."
