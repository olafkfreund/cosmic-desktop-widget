# Configuration Hot-Reload

## Overview

The COSMIC Desktop Widget supports hot-reloading of configuration changes without requiring a restart. When you modify the configuration file, the widget automatically detects the change and updates its appearance and behavior.

## How It Works

The widget uses the `notify` crate to monitor the configuration file at:
```
~/.config/cosmic-desktop-widget/config.toml
```

When a change is detected:

1. **Debouncing**: Multiple rapid changes (within 100ms) are coalesced into a single reload
2. **Validation**: The new configuration is loaded and validated
3. **Widget Recreation**: All widgets are recreated with new settings
4. **Theme Update**: If the theme changed, the renderer is updated
5. **Surface Resize**: If dimensions or position changed, the Layer Shell surface is recreated
6. **Immediate Redraw**: The widget is redrawn with the new configuration

## What Can Be Changed

### Instant Updates (No Surface Recreation)
- Theme changes
- Widget enable/disable
- Widget configuration (city, API key, etc.)
- Background opacity
- Padding and spacing

### Requires Surface Recreation
- Panel width and height
- Position on screen
- Margins

## Testing Hot-Reload

### Manual Testing

1. Start the widget with logging enabled:
   ```bash
   RUST_LOG=info cargo run
   ```

2. In another terminal, edit the config:
   ```bash
   nano ~/.config/cosmic-desktop-widget/config.toml
   ```

3. Make a change (e.g., change width from 450 to 500)

4. Save the file

5. Watch the widget terminal for:
   ```
   Config reload triggered by file change
   Configuration reload complete
   ```

### Automated Testing

Use the provided test script:
```bash
./test_config_reload.sh
```

This script will:
- Backup your config
- Modify panel dimensions
- Wait for reload
- Restore original config

## Implementation Details

### Architecture

```
notify::RecommendedWatcher
    ↓ (file system events)
ConfigWatcher::try_recv()
    ↓ (mpsc channel)
Timer callback in main event loop
    ↓ (polls for events)
DesktopWidget::reload_config()
    ↓
1. Config::load() - Load and validate
2. Recreate widgets
3. Update theme
4. Resize surface if needed
5. Force redraw
```

### Debouncing

Text editors often save files multiple times in quick succession. To avoid triggering multiple reloads, the config watcher implements debouncing:

```rust
const DEBOUNCE_DURATION: Duration = Duration::from_millis(100);
```

Events within 100ms of the previous event are ignored.

### Error Handling

If the config file contains errors after editing:

1. The error is logged
2. The current configuration is preserved
3. The widget continues running normally

Example error log:
```
Failed to load config during reload, keeping current config:
  invalid value: integer `0`, expected u32 > 0 at line 2 column 9
```

### Performance

Hot-reload is implemented efficiently:

- **File watching**: Uses OS-native file system events (inotify on Linux)
- **Polling**: Integrated into existing timer callback (no additional threads)
- **Minimal overhead**: < 1ms to check for reload events
- **On-demand**: Surface recreation only when dimensions/position change

## Configuration Examples

### Change Theme

```toml
[panel]
theme = "light"  # Change from "cosmic_dark" to "light"
```

Result: Immediate theme change, no surface recreation.

### Resize Panel

```toml
[panel]
width = 500   # Changed from 450
height = 200  # Changed from 180
```

Result: Surface destroyed and recreated with new size.

### Adjust Opacity

```toml
[panel]
background_opacity = 0.7  # Add transparency
```

Result: Immediate opacity change, no surface recreation.

### Add/Remove Widgets

```toml
[[widgets]]
widget_type = "clock"
enabled = false  # Disable clock widget

[[widgets]]
widget_type = "weather"
enabled = true   # Enable weather widget
```

Result: Widgets recreated, layout updated.

## Troubleshooting

### Config changes not detected

1. Check file permissions:
   ```bash
   ls -l ~/.config/cosmic-desktop-widget/config.toml
   ```

2. Verify the watcher initialized:
   ```
   Config file watcher initialized
   Config hot-reload enabled
   ```

3. Check for errors in the log:
   ```bash
   RUST_LOG=debug cargo run
   ```

### Widget doesn't update

1. Check for config validation errors in the log
2. Verify the config file syntax with a TOML validator
3. Ensure the file is saved (some editors buffer writes)

### Multiple reloads

If you see multiple reload messages for a single edit, this is normal with some editors. The debouncing should coalesce them, but you might see:
```
Config change debounced
Config reload triggered by file change
```

## Developer Notes

### Adding Reload Support to New Config Fields

When adding new configuration fields:

1. Add the field to `Config` or `PanelConfig` struct
2. Update `DesktopWidget::reload_config()` if special handling is needed
3. Determine if the change requires surface recreation
4. Test with hot-reload enabled

### Surface Recreation Triggers

Add to `reload_config()` checks:
```rust
let needs_recreation = new_config.panel.width != self.config.panel.width
    || new_config.panel.height != self.config.panel.height
    || new_config.panel.position != self.config.panel.position
    || new_config.panel.margin != self.config.panel.margin
    || new_config.your_new_field != self.config.your_new_field;
```

### Testing

Unit tests for the config watcher:
```bash
cargo test --lib config_watcher
```

Integration test (requires running widget):
```bash
./test_config_reload.sh
```

## Future Enhancements

Potential improvements to hot-reload:

1. **Animated transitions**: Smooth transition when resizing
2. **Validation preview**: Check config before applying
3. **Rollback on error**: Automatic revert to last good config
4. **Configuration UI**: Live preview of changes
5. **Per-widget reload**: Only recreate changed widgets
6. **Network config**: Watch remote config files

## References

- [notify crate documentation](https://docs.rs/notify/)
- [Layer Shell specification](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [COSMIC Desktop Widget Configuration](../README.md#configuration)

---

**Last Updated**: 2025-02-05
**Implemented In**: v0.1.0
