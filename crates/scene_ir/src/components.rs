//! ECS-style components that attach to scene graph nodes.
//!
//! Each component is an independent data bag. Nodes gain behavior by
//! composing components — a node with `Transform + Style + Shape` is a
//! visible shape; a node with only `Transform` is an invisible group.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{AssetRef, Color, FontSpec, ImageFit, Stroke, TextAlign, TextVerticalAlign, Vec2};

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Spatial transform in local coordinate space.
/// World-space transforms are computed by composing up the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Transform {
    /// Position relative to parent (or world origin for root nodes).
    #[serde(default)]
    pub position: Vec2,
    /// Rotation in radians (counter-clockwise).
    #[serde(default)]
    pub rotation: f64,
    /// Non-uniform scale.
    #[serde(default = "Vec2::one")]
    pub scale: Vec2,
    /// Anchor point for rotation and scaling, in local coordinates.
    /// (0,0) = top-left, (0.5,0.5) = center.
    #[serde(default = "Vec2::half")]
    pub anchor: Vec2,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            anchor: Vec2::new(0.5, 0.5),
        }
    }
}

impl Transform {
    /// Create a transform positioned at the given coordinates.
    pub fn at(x: f64, y: f64) -> Self {
        Self {
            position: Vec2::new(x, y),
            ..Default::default()
        }
    }
}

// Helper functions for serde defaults (must be free functions for serde).
impl Vec2 {
    pub(crate) fn one() -> Self {
        Self::ONE
    }

    pub(crate) fn half() -> Self {
        Self::new(0.5, 0.5)
    }
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

/// Visual style properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Style {
    /// Fill color. `None` means no fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    /// Stroke configuration. `None` means no stroke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    /// Opacity in [0.0, 1.0].
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// Z-index for draw ordering within siblings.
    #[serde(default)]
    pub z_index: i32,
    /// Whether this node is visible.
    #[serde(default = "default_true")]
    pub visible: bool,
}

fn default_opacity() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            opacity: 1.0,
            z_index: 0,
            visible: true,
        }
    }
}

impl Style {
    /// Convenience: create a style with only a fill color.
    pub fn fill(color: Color) -> Self {
        Self {
            fill: Some(color),
            ..Default::default()
        }
    }

    /// Convenience: create a style with only a stroke.
    pub fn stroke(stroke: Stroke) -> Self {
        Self {
            stroke: Some(stroke),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Shape
// ---------------------------------------------------------------------------

/// Geometric shape definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Shape {
    pub kind: ShapeKind,
}

/// Concrete shape variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeKind {
    Rectangle {
        width: f64,
        height: f64,
        #[serde(default)]
        corner_radius: f64,
    },
    Circle {
        radius: f64,
    },
    Ellipse {
        rx: f64,
        ry: f64,
    },
    Line {
        start: Vec2,
        end: Vec2,
    },
    Polygon {
        points: Vec<Vec2>,
        #[serde(default)]
        closed: bool,
    },
    Path {
        /// SVG path data string (e.g., "M 0 0 L 100 100 Q 200 50 300 100").
        data: String,
    },
    Arc {
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    Arrow {
        start: Vec2,
        end: Vec2,
        #[serde(default = "default_arrow_head_size")]
        head_size: f64,
    },
}

fn default_arrow_head_size() -> f64 {
    10.0
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Text content and typography.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextContent {
    /// The text string to display.
    pub content: String,
    /// Font specification.
    #[serde(default)]
    pub font: FontSpec,
    /// Horizontal alignment.
    #[serde(default)]
    pub align: TextAlign,
    /// Vertical alignment.
    #[serde(default)]
    pub vertical_align: TextVerticalAlign,
    /// Maximum width before wrapping. `None` for no wrapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<f64>,
    /// Line height multiplier.
    #[serde(default = "default_line_height")]
    pub line_height: f64,
}

fn default_line_height() -> f64 {
    1.4
}

impl Default for TextContent {
    fn default() -> Self {
        Self {
            content: String::new(),
            font: FontSpec::default(),
            align: TextAlign::default(),
            vertical_align: TextVerticalAlign::default(),
            max_width: None,
            line_height: 1.4,
        }
    }
}

// ---------------------------------------------------------------------------
// Math Expression (LaTeX)
// ---------------------------------------------------------------------------

/// LaTeX mathematical expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MathExpression {
    /// Raw LaTeX string (e.g., `"\\frac{a}{b} = c"`).
    pub latex: String,
    /// Font size for the rendered expression.
    #[serde(default = "default_math_font_size")]
    pub font_size: f64,
    /// Color of the rendered expression.
    #[serde(default)]
    pub color: Color,
}

fn default_math_font_size() -> f64 {
    36.0
}

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

/// Image component referencing an external asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImageContent {
    /// Reference to the image asset.
    pub asset_ref: AssetRef,
    /// How the image fits within its bounds.
    #[serde(default)]
    pub fit: ImageFit,
    /// Display width. `None` uses the image's natural width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Display height. `None` uses the image's natural height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

// ---------------------------------------------------------------------------
// Code Block
// ---------------------------------------------------------------------------

/// Source code block with syntax highlighting metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CodeBlock {
    /// The source code text.
    pub code: String,
    /// Programming language for syntax highlighting.
    pub language: String,
    /// Font specification (should be monospace).
    #[serde(default = "default_code_font")]
    pub font: FontSpec,
    /// Whether to show line numbers.
    #[serde(default)]
    pub show_line_numbers: bool,
    /// Lines to highlight (1-indexed).
    #[serde(default)]
    pub highlighted_lines: Vec<u32>,
}

fn default_code_font() -> FontSpec {
    FontSpec {
        family: "JetBrains Mono".to_string(),
        size: 18.0,
        weight: crate::types::FontWeight::Regular,
        style: crate::types::FontStyle::Normal,
    }
}

// ---------------------------------------------------------------------------
// Diagram
// ---------------------------------------------------------------------------

/// Diagram definition (Mermaid, PlantUML, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagram {
    /// Diagram source in the specified language.
    pub source: String,
    /// Diagram language/format.
    pub language: DiagramLanguage,
}

/// Supported diagram languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagramLanguage {
    Mermaid,
    PlantUml,
    Graphviz,
    D2,
}

// ---------------------------------------------------------------------------
// Layout constraints
// ---------------------------------------------------------------------------

/// Spatial layout constraint between nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutConstraint {
    /// Align this node relative to another.
    AlignTo {
        target: crate::types::NodeId,
        alignment: Alignment,
        #[serde(default)]
        offset: Vec2,
    },
    /// Keep a fixed distance from another node.
    DistanceFrom {
        target: crate::types::NodeId,
        distance: f64,
        direction: Direction,
    },
    /// Center this node within its parent.
    CenterInParent {
        #[serde(default = "default_true")]
        horizontal: bool,
        #[serde(default = "default_true")]
        vertical: bool,
    },
}

/// Alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Cardinal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_default() {
        let t = Transform::default();
        assert_eq!(t.position, Vec2::ZERO);
        assert_eq!(t.rotation, 0.0);
        assert_eq!(t.scale, Vec2::ONE);
    }

    #[test]
    fn test_transform_at() {
        let t = Transform::at(100.0, 200.0);
        assert_eq!(t.position, Vec2::new(100.0, 200.0));
        assert_eq!(t.scale, Vec2::ONE);
    }

    #[test]
    fn test_shape_rectangle_serde() {
        let shape = Shape {
            kind: ShapeKind::Rectangle {
                width: 200.0,
                height: 100.0,
                corner_radius: 8.0,
            },
        };
        let json = serde_json::to_string(&shape).unwrap();
        let deserialized: Shape = serde_json::from_str(&json).unwrap();
        assert_eq!(shape, deserialized);
    }

    #[test]
    fn test_shape_polygon_serde() {
        let shape = Shape {
            kind: ShapeKind::Polygon {
                points: vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(100.0, 0.0),
                    Vec2::new(50.0, 86.6),
                ],
                closed: true,
            },
        };
        let json = serde_json::to_string_pretty(&shape).unwrap();
        let deserialized: Shape = serde_json::from_str(&json).unwrap();
        assert_eq!(shape, deserialized);
    }

    #[test]
    fn test_style_fill_convenience() {
        let s = Style::fill(Color::WHITE);
        assert_eq!(s.fill, Some(Color::WHITE));
        assert_eq!(s.opacity, 1.0);
        assert!(s.visible);
    }

    #[test]
    fn test_text_content_default() {
        let t = TextContent::default();
        assert_eq!(t.content, "");
        assert_eq!(t.line_height, 1.4);
    }
}
