//! Layout system for positioning widgets within the container

/// Widget position configuration
#[derive(Debug, Clone, Copy)]
pub struct WidgetPosition {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

/// Layout manager for arranging widgets
pub struct LayoutManager {
    container_width: u32,
    container_height: u32,
    padding: f32,
    spacing: f32,
    direction: LayoutDirection,
}

impl LayoutManager {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            container_width: width,
            container_height: height,
            padding: 20.0,
            spacing: 10.0,
            direction: LayoutDirection::Vertical,
        }
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Calculate positions for a list of widgets with given heights
    pub fn calculate_positions(&self, widget_heights: &[f32]) -> Vec<WidgetPosition> {
        let mut positions = Vec::new();
        let available_width = self.container_width as f32 - (self.padding * 2.0);

        match self.direction {
            LayoutDirection::Vertical => {
                let mut current_y = self.padding;
                for &height in widget_heights {
                    positions.push(WidgetPosition {
                        x: self.padding,
                        y: current_y,
                        width: available_width,
                        height,
                    });
                    current_y += height + self.spacing;
                }
            }
            LayoutDirection::Horizontal => {
                let total_widgets = widget_heights.len();
                let widget_width = (available_width - (self.spacing * (total_widgets - 1) as f32))
                    / total_widgets as f32;
                let mut current_x = self.padding;
                for &height in widget_heights {
                    positions.push(WidgetPosition {
                        x: current_x,
                        y: self.padding,
                        width: widget_width,
                        height,
                    });
                    current_x += widget_width + self.spacing;
                }
            }
        }

        positions
    }

    /// Get position for clock widget
    pub fn clock_position(&self, show_weather: bool) -> WidgetPosition {
        if show_weather {
            WidgetPosition {
                x: self.padding,
                y: self.padding,
                width: self.container_width as f32 - (self.padding * 2.0),
                height: 40.0,
            }
        } else {
            // Center clock if no weather
            WidgetPosition {
                x: self.padding,
                y: self.container_height as f32 / 2.0 - 20.0,
                width: self.container_width as f32 - (self.padding * 2.0),
                height: 40.0,
            }
        }
    }

    /// Get position for weather widget
    pub fn weather_position(&self, show_clock: bool) -> WidgetPosition {
        if show_clock {
            WidgetPosition {
                x: self.padding,
                y: self.padding + 40.0 + self.spacing,
                width: self.container_width as f32 - (self.padding * 2.0),
                height: 30.0,
            }
        } else {
            // Center weather if no clock
            WidgetPosition {
                x: self.padding,
                y: self.container_height as f32 / 2.0 - 15.0,
                width: self.container_width as f32 - (self.padding * 2.0),
                height: 30.0,
            }
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new(400, 150)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_manager_creation() {
        let layout = LayoutManager::new(400, 150);
        assert_eq!(layout.container_width, 400);
        assert_eq!(layout.container_height, 150);
    }

    #[test]
    fn test_vertical_layout() {
        let layout = LayoutManager::new(400, 200)
            .with_padding(10.0)
            .with_spacing(5.0);

        let positions = layout.calculate_positions(&[30.0, 20.0, 25.0]);

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].y, 10.0);
        assert_eq!(positions[1].y, 45.0); // 10 + 30 + 5
        assert_eq!(positions[2].y, 70.0); // 45 + 20 + 5
    }

    #[test]
    fn test_horizontal_layout() {
        let layout = LayoutManager::new(400, 100)
            .with_padding(10.0)
            .with_spacing(10.0)
            .with_direction(LayoutDirection::Horizontal);

        let positions = layout.calculate_positions(&[50.0, 50.0]);

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].x, 10.0);
        // Second widget should be offset by first widget width + spacing
        // Available width: 400 - 20 = 380
        // Widget width: (380 - 10) / 2 = 185
        // Second widget x: 10 + 185 + 10 = 205
        assert_eq!(positions[1].x, 205.0);
    }

    #[test]
    fn test_clock_position_with_weather() {
        let layout = LayoutManager::new(400, 150);
        let pos = layout.clock_position(true);

        assert_eq!(pos.x, 20.0);
        assert_eq!(pos.y, 20.0);
        assert_eq!(pos.height, 40.0);
    }

    #[test]
    fn test_clock_position_without_weather() {
        let layout = LayoutManager::new(400, 150);
        let pos = layout.clock_position(false);

        // Should be centered vertically
        assert_eq!(pos.y, 55.0); // 150/2 - 20
    }

    #[test]
    fn test_weather_position_with_clock() {
        let layout = LayoutManager::new(400, 150);
        let pos = layout.weather_position(true);

        // Should be below clock
        assert_eq!(pos.y, 70.0); // 20 + 40 + 10
    }

    #[test]
    fn test_weather_position_without_clock() {
        let layout = LayoutManager::new(400, 150);
        let pos = layout.weather_position(false);

        // Should be centered vertically
        assert_eq!(pos.y, 60.0); // 150/2 - 15
    }
}
