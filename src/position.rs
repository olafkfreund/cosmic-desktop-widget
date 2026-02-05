//! Widget position configuration and Layer Shell anchor mapping

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use std::fmt;
use std::str::FromStr;

/// Widget position on the screen
///
/// This enum defines all 9 possible positions for the widget on the screen.
/// Positions are serialized as kebab-case strings (e.g., "top-left").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    /// Top-left corner
    TopLeft,
    /// Top edge, horizontally centered
    TopCenter,
    /// Top-right corner
    TopRight,
    /// Left edge, vertically centered
    CenterLeft,
    /// Screen center (both axes)
    Center,
    /// Right edge, vertically centered
    CenterRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom edge, horizontally centered
    BottomCenter,
    /// Bottom-right corner
    BottomRight,
}

impl Position {
    /// Convert position to Layer Shell anchor flags
    ///
    /// # Layer Shell Anchor Behavior
    ///
    /// Anchors determine which edges of the screen the widget is attached to:
    /// - No anchors (empty): Widget is centered
    /// - Single anchor: Widget is centered on that edge
    /// - Two anchors (corner): Widget is positioned at that corner
    ///
    /// # Examples
    ///
    /// ```
    /// use cosmic_desktop_widget::Position;
    /// use smithay_client_toolkit::shell::wlr_layer::Anchor;
    ///
    /// // Top-left corner uses both TOP and LEFT anchors
    /// let anchor = Position::TopLeft.to_anchor();
    /// assert_eq!(anchor, Anchor::TOP | Anchor::LEFT);
    ///
    /// // Top-center uses only TOP anchor (centered horizontally)
    /// let anchor = Position::TopCenter.to_anchor();
    /// assert_eq!(anchor, Anchor::TOP);
    ///
    /// // Center position uses no anchors (centered on both axes)
    /// let anchor = Position::Center.to_anchor();
    /// assert_eq!(anchor, Anchor::empty());
    /// ```
    pub fn to_anchor(self) -> Anchor {
        match self {
            Position::TopLeft => Anchor::TOP | Anchor::LEFT,
            Position::TopCenter => Anchor::TOP,
            Position::TopRight => Anchor::TOP | Anchor::RIGHT,
            Position::CenterLeft => Anchor::LEFT,
            Position::Center => Anchor::empty(),
            Position::CenterRight => Anchor::RIGHT,
            Position::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
            Position::BottomCenter => Anchor::BOTTOM,
            Position::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
        }
    }

    /// Convert to kebab-case string representation
    ///
    /// This is the format used in configuration files.
    ///
    /// # Examples
    ///
    /// ```
    /// use cosmic_desktop_widget::Position;
    ///
    /// assert_eq!(Position::TopLeft.as_str(), "top-left");
    /// assert_eq!(Position::Center.as_str(), "center");
    /// assert_eq!(Position::BottomRight.as_str(), "bottom-right");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Position::TopLeft => "top-left",
            Position::TopCenter => "top-center",
            Position::TopRight => "top-right",
            Position::CenterLeft => "center-left",
            Position::Center => "center",
            Position::CenterRight => "center-right",
            Position::BottomLeft => "bottom-left",
            Position::BottomCenter => "bottom-center",
            Position::BottomRight => "bottom-right",
        }
    }

    /// Get all valid position strings
    ///
    /// Useful for validation error messages and documentation.
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "top-left",
            "top-center",
            "top-right",
            "center-left",
            "center",
            "center-right",
            "bottom-left",
            "bottom-center",
            "bottom-right",
        ]
    }

    /// Check if this position is on the top edge
    pub fn is_top(self) -> bool {
        matches!(
            self,
            Position::TopLeft | Position::TopCenter | Position::TopRight
        )
    }

    /// Check if this position is on the bottom edge
    pub fn is_bottom(self) -> bool {
        matches!(
            self,
            Position::BottomLeft | Position::BottomCenter | Position::BottomRight
        )
    }

    /// Check if this position is on the left edge
    pub fn is_left(self) -> bool {
        matches!(
            self,
            Position::TopLeft | Position::CenterLeft | Position::BottomLeft
        )
    }

    /// Check if this position is on the right edge
    pub fn is_right(self) -> bool {
        matches!(
            self,
            Position::TopRight | Position::CenterRight | Position::BottomRight
        )
    }

    /// Check if this position is centered on any axis
    pub fn is_centered(self) -> bool {
        matches!(
            self,
            Position::TopCenter
                | Position::CenterLeft
                | Position::Center
                | Position::CenterRight
                | Position::BottomCenter
        )
    }
}

impl FromStr for Position {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "top-left" => Ok(Position::TopLeft),
            "top-center" => Ok(Position::TopCenter),
            "top-right" => Ok(Position::TopRight),
            "center-left" => Ok(Position::CenterLeft),
            "center" => Ok(Position::Center),
            "center-right" => Ok(Position::CenterRight),
            "bottom-left" => Ok(Position::BottomLeft),
            "bottom-center" => Ok(Position::BottomCenter),
            "bottom-right" => Ok(Position::BottomRight),
            _ => bail!(
                "Invalid position '{}', must be one of: {}",
                s,
                Position::all_variants().join(", ")
            ),
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for Position {
    fn default() -> Self {
        Position::TopRight
    }
}

// Serialize as kebab-case string
impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// Deserialize from kebab-case string
impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Position::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_from_str() {
        assert_eq!(Position::from_str("top-left").unwrap(), Position::TopLeft);
        assert_eq!(
            Position::from_str("top-center").unwrap(),
            Position::TopCenter
        );
        assert_eq!(Position::from_str("top-right").unwrap(), Position::TopRight);
        assert_eq!(
            Position::from_str("center-left").unwrap(),
            Position::CenterLeft
        );
        assert_eq!(Position::from_str("center").unwrap(), Position::Center);
        assert_eq!(
            Position::from_str("center-right").unwrap(),
            Position::CenterRight
        );
        assert_eq!(
            Position::from_str("bottom-left").unwrap(),
            Position::BottomLeft
        );
        assert_eq!(
            Position::from_str("bottom-center").unwrap(),
            Position::BottomCenter
        );
        assert_eq!(
            Position::from_str("bottom-right").unwrap(),
            Position::BottomRight
        );
    }

    #[test]
    fn test_position_from_str_invalid() {
        assert!(Position::from_str("invalid").is_err());
        assert!(Position::from_str("top_left").is_err());
        assert!(Position::from_str("TOP-LEFT").is_err());
    }

    #[test]
    fn test_position_to_anchor() {
        // Corners use two anchors
        assert_eq!(Position::TopLeft.to_anchor(), Anchor::TOP | Anchor::LEFT);
        assert_eq!(Position::TopRight.to_anchor(), Anchor::TOP | Anchor::RIGHT);
        assert_eq!(
            Position::BottomLeft.to_anchor(),
            Anchor::BOTTOM | Anchor::LEFT
        );
        assert_eq!(
            Position::BottomRight.to_anchor(),
            Anchor::BOTTOM | Anchor::RIGHT
        );

        // Edges use one anchor
        assert_eq!(Position::TopCenter.to_anchor(), Anchor::TOP);
        assert_eq!(Position::BottomCenter.to_anchor(), Anchor::BOTTOM);
        assert_eq!(Position::CenterLeft.to_anchor(), Anchor::LEFT);
        assert_eq!(Position::CenterRight.to_anchor(), Anchor::RIGHT);

        // Center uses no anchors
        assert_eq!(Position::Center.to_anchor(), Anchor::empty());
    }

    #[test]
    fn test_position_as_str() {
        assert_eq!(Position::TopLeft.as_str(), "top-left");
        assert_eq!(Position::Center.as_str(), "center");
        assert_eq!(Position::BottomRight.as_str(), "bottom-right");
    }

    #[test]
    fn test_position_display() {
        assert_eq!(format!("{}", Position::TopLeft), "top-left");
        assert_eq!(format!("{}", Position::Center), "center");
    }

    #[test]
    fn test_position_default() {
        assert_eq!(Position::default(), Position::TopRight);
    }

    #[test]
    fn test_position_serialization() {
        // TOML requires a table structure, so wrap position in a struct
        #[derive(serde::Serialize, serde::Deserialize)]
        struct TestConfig {
            position: Position,
        }

        let config = TestConfig { position: Position::TopLeft };
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("top-left"));

        let deserialized: TestConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.position, Position::TopLeft);
    }

    #[test]
    fn test_position_edge_checks() {
        assert!(Position::TopLeft.is_top());
        assert!(Position::TopLeft.is_left());
        assert!(!Position::TopLeft.is_bottom());
        assert!(!Position::TopLeft.is_right());

        assert!(Position::Center.is_centered());
        assert!(!Position::TopLeft.is_centered());

        assert!(Position::BottomCenter.is_bottom());
        assert!(Position::BottomCenter.is_centered());
    }

    #[test]
    fn test_all_variants() {
        let variants = Position::all_variants();
        assert_eq!(variants.len(), 9);

        // Verify all variants can be parsed
        for variant in variants {
            assert!(Position::from_str(variant).is_ok());
        }
    }
}
