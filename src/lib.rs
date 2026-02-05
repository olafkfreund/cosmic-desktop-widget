//! COSMIC Desktop Widget Library
//!
//! This library provides the core functionality for the desktop widget.
//! It exposes modules for configuration, rendering, theming, layout,
//! widget implementations, and update scheduling.

#![warn(missing_docs)]

pub mod audio;
pub mod config;
pub mod config_watcher;
pub mod error;
pub mod icons;
pub mod input;
pub mod layout;
pub mod metrics;
pub mod panel;
pub mod position;
pub mod render;
pub mod surface;
pub mod text;
pub mod theme;
pub mod update;
pub mod wayland;
pub mod weather;
pub mod widget;

// Re-export commonly used types
pub use config::{
    Config, GradientConfig, Margin, PanelConfig, SoundsConfig, ThemeColors, ThemeConfig,
    ThemeStyle,
};
pub use config_watcher::{ConfigReloadEvent, ConfigWatcher};
pub use error::{ConfigError, WeatherError, WidgetError};
pub use input::{
    button_code_to_mouse_button, execute_action, hit_test_widgets, scroll_to_direction, InputState,
};
pub use layout::{LayoutDirection, LayoutManager, WidgetPosition};
pub use metrics::{CacheMetrics, RenderMetrics, Timer, WidgetMetrics};
pub use panel::{MarginAdjustments, PanelAnchor, PanelDetection, PanelInfo, PanelSize};
pub use position::Position;
pub use theme::{Color, Theme};
pub use update::{UpdateFlags, UpdateScheduler};
pub use audio::{AudioPlayer, SoundConfig, SoundEffect};
pub use text::FontWeight;
pub use widget::{
    ClockWidget, CountdownWidget, DynWidgetFactory, FontSize, MouseButton, ProgressBar,
    ProgressColor, Quote, QuotesWidget, ScrollDirection, SystemMonitorWidget, TextSegment,
    WeatherData, WeatherWidget, Widget, WidgetAction, WidgetConfig, WidgetContent, WidgetFactory,
    WidgetInfo, WidgetInstance, WidgetRegistry,
};
