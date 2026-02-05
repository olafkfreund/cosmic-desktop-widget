//! Widget trait system for extensible widget development
//!
//! This module defines the core traits that all widgets must implement.
//! New widgets can be added by implementing these traits.

use std::time::Duration;

/// Mouse button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
    /// Other buttons
    Other(u8),
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Scroll up
    Up,
    /// Scroll down
    Down,
    /// Scroll left
    Left,
    /// Scroll right
    Right,
}

/// Actions that a widget can request in response to interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetAction {
    /// Open a URL in the default browser
    OpenUrl(String),
    /// Run a shell command
    RunCommand(String),
    /// Advance to the next item (quotes, news, etc.)
    NextItem,
    /// Go to the previous item
    PreviousItem,
    /// Toggle a state (play/pause, etc.)
    Toggle,
    /// Custom action with string identifier
    Custom(String),
    /// No action
    None,
}

/// Information about a widget for layout purposes
#[derive(Debug, Clone)]
pub struct WidgetInfo {
    /// Unique identifier for this widget type
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Preferred height in pixels (layout hint)
    pub preferred_height: f32,
    /// Minimum height in pixels
    pub min_height: f32,
    /// Whether this widget should expand to fill available space
    pub expand: bool,
}

/// Content to be rendered by a widget
#[derive(Debug, Clone)]
pub enum WidgetContent {
    /// Single line of text with font size
    Text { text: String, size: FontSize },
    /// Multiple lines of text
    MultiLine { lines: Vec<(String, FontSize)> },
    /// Text with an icon (future)
    IconText {
        icon: String,
        text: String,
        size: FontSize,
    },
    /// Progress bar (like the minute progress)
    Progress {
        value: f32, // 0.0 to 1.0
        label: Option<String>,
    },
    /// Empty/nothing to render
    Empty,
}

/// Font size hint for rendering
#[derive(Debug, Clone, Copy)]
pub enum FontSize {
    /// Large text (primary content like clock)
    Large,
    /// Medium text (secondary content)
    Medium,
    /// Small text (labels, status)
    Small,
    /// Custom size in pixels
    Custom(f32),
}

impl FontSize {
    /// Convert to pixels based on container height
    pub fn to_pixels(&self, container_height: f32) -> f32 {
        match self {
            FontSize::Large => container_height * 0.5,
            FontSize::Medium => container_height * 0.25,
            FontSize::Small => container_height * 0.15,
            FontSize::Custom(px) => *px,
        }
    }
}

/// Core trait that all widgets must implement
pub trait Widget: Send {
    /// Get widget metadata
    fn info(&self) -> WidgetInfo;

    /// Update widget state (called periodically)
    fn update(&mut self);

    /// Get content to render
    fn content(&self) -> WidgetContent;

    /// How often this widget needs updates
    fn update_interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    /// Whether the widget is ready to display
    fn is_ready(&self) -> bool {
        true
    }

    /// Get error message if widget is in error state
    fn error(&self) -> Option<&str> {
        None
    }

    // === Interaction Methods (Optional) ===

    /// Whether this widget accepts pointer interactions
    ///
    /// Widgets that don't handle interactions should return false (default).
    /// Interactive widgets (quotes, news, media controls) should return true.
    fn is_interactive(&self) -> bool {
        false
    }

    /// Handle mouse button click
    ///
    /// Called when the user clicks on this widget's area.
    /// Returns an optional action to be executed by the application.
    ///
    /// # Arguments
    /// * `button` - Which mouse button was clicked
    /// * `x` - X coordinate relative to widget area (0.0 = left edge, 1.0 = right edge)
    /// * `y` - Y coordinate relative to widget area (0.0 = top edge, 1.0 = bottom edge)
    fn on_click(&mut self, _button: MouseButton, _x: f32, _y: f32) -> Option<WidgetAction> {
        None
    }

    /// Handle scroll wheel input
    ///
    /// Called when the user scrolls over this widget's area.
    /// Returns an optional action to be executed by the application.
    ///
    /// # Arguments
    /// * `direction` - Scroll direction
    /// * `x` - X coordinate relative to widget area
    /// * `y` - Y coordinate relative to widget area
    fn on_scroll(&mut self, _direction: ScrollDirection, _x: f32, _y: f32) -> Option<WidgetAction> {
        None
    }

    /// Handle pointer enter event
    ///
    /// Called when the pointer enters this widget's area.
    /// Useful for showing hover effects or tooltips.
    fn on_pointer_enter(&mut self) {}

    /// Handle pointer leave event
    ///
    /// Called when the pointer leaves this widget's area.
    /// Useful for clearing hover effects.
    fn on_pointer_leave(&mut self) {}
}

/// Configuration for a widget instance
pub trait WidgetConfig: Default + Clone {
    /// Widget type identifier
    fn widget_type() -> &'static str;

    /// Validate configuration
    fn validate(&self) -> Result<(), String>;
}

/// Factory for creating widget instances
pub trait WidgetFactory {
    /// The widget type this factory creates
    type Widget: Widget;
    /// Configuration type
    type Config: WidgetConfig;

    /// Create a new widget instance
    fn create(config: &Self::Config) -> Self::Widget;

    /// Widget type identifier
    fn widget_type() -> &'static str {
        Self::Config::widget_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_size_to_pixels() {
        let height = 100.0;
        assert!((FontSize::Large.to_pixels(height) - 50.0).abs() < 0.01);
        assert!((FontSize::Medium.to_pixels(height) - 25.0).abs() < 0.01);
        assert!((FontSize::Small.to_pixels(height) - 15.0).abs() < 0.01);
        assert!((FontSize::Custom(32.0).to_pixels(height) - 32.0).abs() < 0.01);
    }
}
