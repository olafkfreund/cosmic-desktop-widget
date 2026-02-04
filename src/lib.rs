//! COSMIC Desktop Widget Library
//!
//! This library provides the core functionality for the desktop widget.
//! It exposes modules for configuration, rendering, theming, layout,
//! widget implementations, and update scheduling.

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod layout;
pub mod metrics;
pub mod render;
pub mod text;
pub mod theme;
pub mod update;
pub mod wayland;
pub mod weather;
pub mod widget;

// Re-export commonly used types
pub use config::{Config, Margin};
pub use error::{ConfigError, WeatherError, WidgetError};
pub use layout::{LayoutDirection, LayoutManager, WidgetPosition};
pub use metrics::{CacheMetrics, RenderMetrics, Timer, WidgetMetrics};
pub use theme::{Color, Theme};
pub use update::{UpdateFlags, UpdateScheduler};
pub use widget::{ClockWidget, WeatherData, WeatherWidget};
