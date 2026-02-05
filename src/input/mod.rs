//! Input handling for widget interactions
//!
//! This module provides pointer event handling, hit-testing, and action execution
//! for interactive widgets.

use crate::widget::{MouseButton, ScrollDirection, Widget, WidgetAction};
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

/// Input state tracker for pointer events
pub struct InputState {
    /// Last known pointer position (absolute surface coordinates)
    pointer_x: f64,
    pointer_y: f64,
    /// Whether pointer is currently over the surface
    pointer_entered: bool,
    /// Index of widget currently under pointer (if any)
    hovered_widget: Option<usize>,
}

impl InputState {
    /// Create a new input state tracker
    pub fn new() -> Self {
        Self {
            pointer_x: 0.0,
            pointer_y: 0.0,
            pointer_entered: false,
            hovered_widget: None,
        }
    }

    /// Update pointer position
    pub fn update_position(&mut self, x: f64, y: f64) {
        self.pointer_x = x;
        self.pointer_y = y;
    }

    /// Mark pointer as entered
    pub fn pointer_enter(&mut self) {
        self.pointer_entered = true;
        debug!("Pointer entered surface");
    }

    /// Mark pointer as left
    pub fn pointer_leave(&mut self) {
        self.pointer_entered = false;
        self.hovered_widget = None;
        debug!("Pointer left surface");
    }

    /// Get current pointer position
    pub fn pointer_position(&self) -> (f64, f64) {
        (self.pointer_x, self.pointer_y)
    }

    /// Check if pointer is over the surface
    pub fn is_pointer_over(&self) -> bool {
        self.pointer_entered
    }

    /// Update hovered widget and send enter/leave events
    pub fn update_hover(&mut self, widget_index: Option<usize>, widgets: &mut [Box<dyn Widget>]) {
        if self.hovered_widget == widget_index {
            return; // No change
        }

        // Send leave event to previous widget
        if let Some(old_index) = self.hovered_widget {
            if let Some(widget) = widgets.get_mut(old_index) {
                if widget.is_interactive() {
                    widget.on_pointer_leave();
                    debug!(widget_index = old_index, "Pointer left widget");
                }
            }
        }

        // Send enter event to new widget
        if let Some(new_index) = widget_index {
            if let Some(widget) = widgets.get_mut(new_index) {
                if widget.is_interactive() {
                    widget.on_pointer_enter();
                    debug!(widget_index = new_index, "Pointer entered widget");
                }
            }
        }

        self.hovered_widget = widget_index;
    }

    /// Get currently hovered widget index
    pub fn hovered_widget(&self) -> Option<usize> {
        self.hovered_widget
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hit-test to find which widget is at the given coordinates
///
/// Returns the index of the widget that was hit, or None if no widget was hit.
///
/// # Arguments
/// * `x` - X coordinate in surface space (pixels from left)
/// * `y` - Y coordinate in surface space (pixels from top)
/// * `widgets` - Slice of widgets with their layout positions
/// * `widget_positions` - Layout positions for each widget (y_offset, height)
///
/// Note: Currently assumes vertical stacking. Future versions should support
/// more complex layouts.
pub fn hit_test_widgets(
    x: f64,
    y: f64,
    widgets: &[Box<dyn Widget>],
    widget_positions: &[(f32, f32)], // (y_offset, height)
) -> Option<usize> {
    if widgets.len() != widget_positions.len() {
        warn!(
            widgets = widgets.len(),
            positions = widget_positions.len(),
            "Widget count mismatch in hit test"
        );
        return None;
    }

    for (i, ((y_offset, height), widget)) in widget_positions.iter().zip(widgets.iter()).enumerate()
    {
        // Only test interactive widgets
        if !widget.is_interactive() {
            continue;
        }

        let y_start = *y_offset as f64;
        let y_end = y_start + *height as f64;

        if y >= y_start && y < y_end {
            debug!(
                widget_index = i,
                widget_id = widget.info().id,
                x = x,
                y = y,
                "Hit test: widget at position"
            );
            return Some(i);
        }
    }

    None
}

/// Execute a widget action
///
/// Performs the requested action (opening URLs, running commands, etc.)
/// with proper error handling and logging.
pub fn execute_action(action: WidgetAction) -> Result<()> {
    match action {
        WidgetAction::OpenUrl(url) => {
            info!(url = %url, "Executing action: OpenUrl");
            open_url(&url)?;
        }
        WidgetAction::RunCommand(command) => {
            info!(command = %command, "Executing action: RunCommand");
            run_command(&command)?;
        }
        WidgetAction::NextItem => {
            debug!("Executing action: NextItem (handled by widget)");
        }
        WidgetAction::PreviousItem => {
            debug!("Executing action: PreviousItem (handled by widget)");
        }
        WidgetAction::Toggle => {
            debug!("Executing action: Toggle (handled by widget)");
        }
        WidgetAction::Custom(action) => {
            debug!(action = %action, "Executing action: Custom");
        }
        WidgetAction::None => {
            debug!("No action to execute");
        }
    }
    Ok(())
}

/// Open a URL in the default browser
fn open_url(url: &str) -> Result<()> {
    // Use xdg-open on Linux for opening URLs
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .with_context(|| format!("Failed to open URL: {}", url))?;

    info!(url = %url, "Opened URL in browser");
    Ok(())
}

/// Run a shell command
fn run_command(command: &str) -> Result<()> {
    // Execute command via sh for proper shell parsing
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .with_context(|| format!("Failed to run command: {}", command))?;

    info!(command = %command, "Executed command");
    Ok(())
}

/// Convert Wayland button code to MouseButton
pub fn button_code_to_mouse_button(code: u32) -> MouseButton {
    match code {
        0x110 => MouseButton::Left,   // BTN_LEFT
        0x111 => MouseButton::Right,  // BTN_RIGHT
        0x112 => MouseButton::Middle, // BTN_MIDDLE
        other => MouseButton::Other((other & 0xFF) as u8),
    }
}

/// Convert scroll axis value to direction
pub fn scroll_to_direction(value: f64) -> Option<ScrollDirection> {
    if value.abs() < 0.1 {
        return None; // Too small to matter
    }

    // Positive values typically mean scroll down/right
    // Negative values mean scroll up/left
    // (This may vary by compositor)
    if value > 0.0 {
        Some(ScrollDirection::Down)
    } else {
        Some(ScrollDirection::Up)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{FontSize, WidgetContent, WidgetInfo};

    // Mock widget for testing
    struct MockWidget {
        interactive: bool,
        clicked: bool,
    }

    impl Widget for MockWidget {
        fn info(&self) -> WidgetInfo {
            WidgetInfo {
                id: "mock",
                name: "Mock",
                preferred_height: 50.0,
                min_height: 30.0,
                expand: false,
            }
        }

        fn update(&mut self) {}

        fn content(&self) -> WidgetContent {
            WidgetContent::Text {
                text: "Mock".to_string(),
                size: FontSize::Medium,
            }
        }

        fn is_interactive(&self) -> bool {
            self.interactive
        }

        fn on_click(&mut self, _button: MouseButton, _x: f32, _y: f32) -> Option<WidgetAction> {
            self.clicked = true;
            Some(WidgetAction::NextItem)
        }
    }

    #[test]
    fn test_input_state_creation() {
        let state = InputState::new();
        assert!(!state.is_pointer_over());
        assert_eq!(state.pointer_position(), (0.0, 0.0));
    }

    #[test]
    fn test_input_state_enter_leave() {
        let mut state = InputState::new();
        state.pointer_enter();
        assert!(state.is_pointer_over());
        state.pointer_leave();
        assert!(!state.is_pointer_over());
    }

    #[test]
    fn test_input_state_position_update() {
        let mut state = InputState::new();
        state.update_position(100.0, 200.0);
        assert_eq!(state.pointer_position(), (100.0, 200.0));
    }

    #[test]
    fn test_hit_test_widgets() {
        let widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(MockWidget {
                interactive: true,
                clicked: false,
            }),
            Box::new(MockWidget {
                interactive: true,
                clicked: false,
            }),
        ];

        let positions = vec![
            (0.0, 50.0),  // Widget 0: y=0-50
            (50.0, 50.0), // Widget 1: y=50-100
        ];

        // Test hit on first widget
        assert_eq!(hit_test_widgets(10.0, 25.0, &widgets, &positions), Some(0));

        // Test hit on second widget
        assert_eq!(hit_test_widgets(10.0, 75.0, &widgets, &positions), Some(1));

        // Test miss
        assert_eq!(hit_test_widgets(10.0, 150.0, &widgets, &positions), None);
    }

    #[test]
    fn test_hit_test_skip_non_interactive() {
        let widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(MockWidget {
                interactive: false,
                clicked: false,
            }),
            Box::new(MockWidget {
                interactive: true,
                clicked: false,
            }),
        ];

        let positions = vec![(0.0, 50.0), (50.0, 50.0)];

        // Should skip non-interactive widget 0
        assert_eq!(hit_test_widgets(10.0, 25.0, &widgets, &positions), None);

        // Should hit interactive widget 1
        assert_eq!(hit_test_widgets(10.0, 75.0, &widgets, &positions), Some(1));
    }

    #[test]
    fn test_button_code_conversion() {
        assert_eq!(button_code_to_mouse_button(0x110), MouseButton::Left);
        assert_eq!(button_code_to_mouse_button(0x111), MouseButton::Right);
        assert_eq!(button_code_to_mouse_button(0x112), MouseButton::Middle);
    }

    #[test]
    fn test_scroll_direction_conversion() {
        assert_eq!(scroll_to_direction(10.0), Some(ScrollDirection::Down));
        assert_eq!(scroll_to_direction(-10.0), Some(ScrollDirection::Up));
        assert_eq!(scroll_to_direction(0.05), None); // Too small
    }

    #[test]
    fn test_execute_action_none() {
        let result = execute_action(WidgetAction::None);
        assert!(result.is_ok());
    }
}
