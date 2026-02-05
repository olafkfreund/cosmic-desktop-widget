//! COSMIC Panel detection for avoiding overlap
//!
//! Reads COSMIC Desktop panel configuration to determine where panels are
//! positioned and their sizes, allowing widgets to avoid overlap.

use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Panel anchor position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelAnchor {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// Panel size preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelSize {
    XS,
    #[default]
    S,
    M,
    L,
    XL,
}

impl PanelSize {
    /// Convert panel size to approximate pixel height/width
    pub fn to_pixels(self) -> i32 {
        match self {
            PanelSize::XS => 28,
            PanelSize::S => 32,
            PanelSize::M => 36,
            PanelSize::L => 40,
            PanelSize::XL => 48,
        }
    }
}

/// Information about a detected panel
#[derive(Debug, Clone, Default)]
pub struct PanelInfo {
    pub anchor: PanelAnchor,
    pub size: PanelSize,
    pub exclusive_zone: bool,
    pub margin: i32,
}

impl PanelInfo {
    /// Get the total space reserved by this panel (size + margin)
    pub fn reserved_space(&self) -> i32 {
        if self.exclusive_zone {
            self.size.to_pixels() + self.margin
        } else {
            0
        }
    }
}

/// Detected panels on the system
#[derive(Debug, Clone, Default)]
pub struct PanelDetection {
    pub panels: Vec<PanelInfo>,
}

impl PanelDetection {
    /// Detect panels by reading COSMIC panel configuration
    pub fn detect() -> Self {
        let mut detection = Self::default();

        // Try to read COSMIC panel config
        if let Some(panel) = Self::read_cosmic_panel() {
            debug!(
                anchor = ?panel.anchor,
                size = ?panel.size,
                reserved = panel.reserved_space(),
                "Detected COSMIC panel"
            );
            detection.panels.push(panel);
        }

        // Check for dock (separate panel config)
        if let Some(dock) = Self::read_cosmic_dock() {
            debug!(
                anchor = ?dock.anchor,
                size = ?dock.size,
                reserved = dock.reserved_space(),
                "Detected COSMIC dock"
            );
            detection.panels.push(dock);
        }

        if detection.panels.is_empty() {
            debug!("No panels detected, using defaults");
        }

        detection
    }

    /// Read the main COSMIC panel configuration
    fn read_cosmic_panel() -> Option<PanelInfo> {
        let config_dir = dirs::config_dir()?;
        let panel_dir = config_dir.join("cosmic/com.system76.CosmicPanel.Panel/v1");

        Self::read_panel_from_dir(&panel_dir)
    }

    /// Read the COSMIC dock configuration
    fn read_cosmic_dock() -> Option<PanelInfo> {
        let config_dir = dirs::config_dir()?;
        let dock_dir = config_dir.join("cosmic/com.system76.CosmicPanel.Dock/v1");

        Self::read_panel_from_dir(&dock_dir)
    }

    /// Read panel info from a config directory
    fn read_panel_from_dir(dir: &PathBuf) -> Option<PanelInfo> {
        if !dir.exists() {
            return None;
        }

        let anchor = Self::read_file_content(dir.join("anchor"))
            .and_then(|s| Self::parse_anchor(&s))
            .unwrap_or_default();

        let size = Self::read_file_content(dir.join("size"))
            .and_then(|s| Self::parse_size(&s))
            .unwrap_or_default();

        let exclusive_zone = Self::read_file_content(dir.join("exclusive_zone"))
            .map(|s| s.trim() == "true")
            .unwrap_or(true);

        let margin = Self::read_file_content(dir.join("margin"))
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        Some(PanelInfo {
            anchor,
            size,
            exclusive_zone,
            margin,
        })
    }

    /// Read file content as string
    fn read_file_content(path: PathBuf) -> Option<String> {
        fs::read_to_string(&path)
            .map_err(|e| {
                debug!(path = %path.display(), error = %e, "Failed to read panel config file");
                e
            })
            .ok()
    }

    /// Parse anchor string
    fn parse_anchor(s: &str) -> Option<PanelAnchor> {
        match s.trim() {
            "Top" => Some(PanelAnchor::Top),
            "Bottom" => Some(PanelAnchor::Bottom),
            "Left" => Some(PanelAnchor::Left),
            "Right" => Some(PanelAnchor::Right),
            other => {
                warn!(value = other, "Unknown panel anchor");
                None
            }
        }
    }

    /// Parse size string
    fn parse_size(s: &str) -> Option<PanelSize> {
        match s.trim() {
            "XS" => Some(PanelSize::XS),
            "S" => Some(PanelSize::S),
            "M" => Some(PanelSize::M),
            "L" => Some(PanelSize::L),
            "XL" => Some(PanelSize::XL),
            other => {
                warn!(value = other, "Unknown panel size");
                None
            }
        }
    }

    /// Get margin adjustments needed to avoid all panels
    pub fn margin_adjustments(&self) -> MarginAdjustments {
        let mut adjustments = MarginAdjustments::default();

        for panel in &self.panels {
            let space = panel.reserved_space();
            match panel.anchor {
                PanelAnchor::Top => adjustments.top = adjustments.top.max(space),
                PanelAnchor::Bottom => adjustments.bottom = adjustments.bottom.max(space),
                PanelAnchor::Left => adjustments.left = adjustments.left.max(space),
                PanelAnchor::Right => adjustments.right = adjustments.right.max(space),
            }
        }

        // Add a small gap between panel and widget
        const PANEL_GAP: i32 = 8;
        if adjustments.top > 0 {
            adjustments.top += PANEL_GAP;
        }
        if adjustments.bottom > 0 {
            adjustments.bottom += PANEL_GAP;
        }
        if adjustments.left > 0 {
            adjustments.left += PANEL_GAP;
        }
        if adjustments.right > 0 {
            adjustments.right += PANEL_GAP;
        }

        adjustments
    }
}

/// Margin adjustments to avoid panels
#[derive(Debug, Clone, Copy, Default)]
pub struct MarginAdjustments {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_size_to_pixels() {
        assert_eq!(PanelSize::XS.to_pixels(), 28);
        assert_eq!(PanelSize::S.to_pixels(), 32);
        assert_eq!(PanelSize::M.to_pixels(), 36);
        assert_eq!(PanelSize::L.to_pixels(), 40);
        assert_eq!(PanelSize::XL.to_pixels(), 48);
    }

    #[test]
    fn test_panel_info_reserved_space() {
        let panel = PanelInfo {
            anchor: PanelAnchor::Top,
            size: PanelSize::S,
            exclusive_zone: true,
            margin: 4,
        };
        assert_eq!(panel.reserved_space(), 36); // 32 + 4

        let no_exclusive = PanelInfo {
            exclusive_zone: false,
            ..panel
        };
        assert_eq!(no_exclusive.reserved_space(), 0);
    }

    #[test]
    fn test_parse_anchor() {
        assert_eq!(PanelDetection::parse_anchor("Top"), Some(PanelAnchor::Top));
        assert_eq!(
            PanelDetection::parse_anchor("Bottom\n"),
            Some(PanelAnchor::Bottom)
        );
        assert_eq!(PanelDetection::parse_anchor("Unknown"), None);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(PanelDetection::parse_size("XS"), Some(PanelSize::XS));
        assert_eq!(PanelDetection::parse_size("M\n"), Some(PanelSize::M));
        assert_eq!(PanelDetection::parse_size("XXL"), None);
    }

    #[test]
    fn test_margin_adjustments() {
        let detection = PanelDetection {
            panels: vec![
                PanelInfo {
                    anchor: PanelAnchor::Top,
                    size: PanelSize::S,
                    exclusive_zone: true,
                    margin: 0,
                },
                PanelInfo {
                    anchor: PanelAnchor::Bottom,
                    size: PanelSize::L,
                    exclusive_zone: true,
                    margin: 4,
                },
            ],
        };

        let adjustments = detection.margin_adjustments();
        assert_eq!(adjustments.top, 32 + 8); // S size + gap
        assert_eq!(adjustments.bottom, 44 + 8); // L size + margin + gap
        assert_eq!(adjustments.left, 0);
        assert_eq!(adjustments.right, 0);
    }
}
