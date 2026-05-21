//! Fundamental primitive types used across the entire scene IR.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Identity types
// ---------------------------------------------------------------------------

/// A helper macro to implement JsonSchema for UUID newtype wrappers.
/// UUIDs are serialized as strings, so the JSON Schema is just `{ "type": "string", "format": "uuid" }`.
macro_rules! impl_uuid_json_schema {
    ($ty:ident, $name:literal) => {
        impl schemars::JsonSchema for $ty {
            fn schema_name() -> String {
                $name.to_string()
            }
            fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
                schemars::schema::SchemaObject {
                    instance_type: Some(schemars::schema::InstanceType::String.into()),
                    format: Some("uuid".to_string()),
                    ..Default::default()
                }
                .into()
            }
        }
    };
}

/// Unique identifier for a node in the scene graph.
/// Uses UUID v7 for stable identity across edits (time-sortable, globally unique).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub Uuid);

impl_uuid_json_schema!(NodeId, "NodeId");

impl NodeId {
    /// Create a new unique NodeId using UUID v7 (time-sortable).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a NodeId from an existing UUID (for deserialization/testing).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create a nil/zero NodeId (useful as sentinel).
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneId(pub Uuid);

impl_uuid_json_schema!(SceneId, "SceneId");

impl SceneId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SceneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub Uuid);

impl_uuid_json_schema!(ProjectId, "ProjectId");

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub Uuid);

impl_uuid_json_schema!(EventId, "EventId");

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Content-addressable hash for caching and deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn new(hash: String) -> Self {
        Self(hash)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Geometric types
// ---------------------------------------------------------------------------

/// 2D vector / point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self::ZERO
    }
}

/// 3D vector (for future 3D scene support and depth ordering).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self::ZERO
    }
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// RGBA color with f64 components in [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from a hex string like "#FF5500" or "#FF550088".
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
                Some(Self::new(r, g, b, a))
            }
            _ => None,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

// ---------------------------------------------------------------------------
// Stroke
// ---------------------------------------------------------------------------

/// Stroke configuration for shape outlines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
        }
    }
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

// ---------------------------------------------------------------------------
// Font
// ---------------------------------------------------------------------------

/// Font specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FontSpec {
    pub family: String,
    pub size: f64,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            family: "Inter".to_string(),
            size: 24.0,
            weight: FontWeight::Regular,
            style: FontStyle::Normal,
        }
    }
}

/// Font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Thin,
    Light,
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

/// Font style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

// ---------------------------------------------------------------------------
// Text alignment
// ---------------------------------------------------------------------------

/// Text horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

/// Text vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextVerticalAlign {
    Top,
    Middle,
    Bottom,
}

impl Default for TextVerticalAlign {
    fn default() -> Self {
        Self::Top
    }
}

// ---------------------------------------------------------------------------
// Image fitting
// ---------------------------------------------------------------------------

/// How an image fits within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Contain,
    Cover,
    Fill,
    None,
}

impl Default for ImageFit {
    fn default() -> Self {
        Self::Contain
    }
}

// ---------------------------------------------------------------------------
// Asset reference
// ---------------------------------------------------------------------------

/// Reference to an external asset (image, font, audio, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct AssetRef {
    /// Unique asset identifier within the project.
    pub asset_id: String,
    /// Relative path to the asset file from the project root.
    pub path: String,
    /// MIME type of the asset.
    pub mime_type: Option<String>,
    /// Content hash for integrity verification.
    pub content_hash: Option<ContentHash>,
}

// ---------------------------------------------------------------------------
// Render output format
// ---------------------------------------------------------------------------

/// Output format for rendered artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Mp4,
    Webm,
    Gif,
    Png,
    Svg,
    ImageSequence,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Mp4
    }
}

// ---------------------------------------------------------------------------
// Duration (in seconds, f64 for sub-frame precision)
// ---------------------------------------------------------------------------

/// Duration in seconds. Uses f64 for sub-frame precision.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct DurationSecs(pub f64);

impl DurationSecs {
    pub const ZERO: Self = Self(0.0);

    pub const fn new(secs: f64) -> Self {
        Self(secs)
    }

    pub fn from_frames(frames: u64, fps: f64) -> Self {
        Self(frames as f64 / fps)
    }

    pub fn to_frames(self, fps: f64) -> u64 {
        (self.0 * fps).round() as u64
    }
}

impl Default for DurationSecs {
    fn default() -> Self {
        Self::ZERO
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_uniqueness() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_node_id_nil() {
        let nil = NodeId::nil();
        assert_eq!(nil.0, Uuid::nil());
    }

    #[test]
    fn test_color_from_hex() {
        let c = Color::from_hex("#FF0000").unwrap();
        assert!((c.r - 1.0).abs() < f64::EPSILON);
        assert!((c.g - 0.0).abs() < f64::EPSILON);
        assert!((c.b - 0.0).abs() < f64::EPSILON);
        assert!((c.a - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_color_from_hex_with_alpha() {
        let c = Color::from_hex("#FF000080").unwrap();
        assert!((c.a - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_color_from_hex_invalid() {
        assert!(Color::from_hex("#GG0000").is_none());
        assert!(Color::from_hex("invalid").is_none());
    }

    #[test]
    fn test_duration_frame_conversion() {
        let d = DurationSecs::from_frames(60, 30.0);
        assert!((d.0 - 2.0).abs() < f64::EPSILON);
        assert_eq!(d.to_frames(30.0), 60);
    }

    #[test]
    fn test_vec2_constants() {
        assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
        assert_eq!(Vec2::ONE, Vec2::new(1.0, 1.0));
    }

    #[test]
    fn test_node_id_serde_roundtrip() {
        let id = NodeId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_color_serde_roundtrip() {
        let c = Color::rgb(0.5, 0.7, 0.3);
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: Color = serde_json::from_str(&json).unwrap();
        assert_eq!(c, deserialized);
    }
}
