//! Unified timeline: keyframe tracks for continuous properties +
//! animation events for discrete transitions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{Color, DurationSecs, EventId, NodeId, Vec2};

// ---------------------------------------------------------------------------
// Timeline
// ---------------------------------------------------------------------------

/// The animation timeline for a scene.
///
/// Combines two paradigms:
/// - **Keyframe tracks**: continuous property interpolation (like After Effects / CSS)
/// - **Animation events**: discrete transition blocks (like Manim's `self.play()`)
///
/// Both coexist on the same timeline and are resolved in temporal order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Timeline {
    /// Total duration of the timeline.
    pub duration: DurationSecs,

    /// Continuous property animation tracks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<KeyframeTrack>,

    /// Discrete animation events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<AnimationEvent>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            duration: DurationSecs::ZERO,
            tracks: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl Timeline {
    /// Create a timeline with the given duration.
    pub fn with_duration(duration: DurationSecs) -> Self {
        Self {
            duration,
            ..Default::default()
        }
    }

    /// Get all events sorted by start time.
    pub fn sorted_events(&self) -> Vec<&AnimationEvent> {
        let mut events: Vec<&AnimationEvent> = self.events.iter().collect();
        events.sort_by(|a, b| a.start_time.0.partial_cmp(&b.start_time.0).unwrap());
        events
    }
}

// ---------------------------------------------------------------------------
// Keyframe Track
// ---------------------------------------------------------------------------

/// A track of keyframes animating a single property on a single node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KeyframeTrack {
    /// The node this track targets.
    pub target_node: NodeId,

    /// Which property to animate.
    pub property: AnimatableProperty,

    /// Ordered keyframes (must be sorted by time).
    pub keyframes: Vec<Keyframe>,
}

/// A single keyframe: a (time, value) pair with an easing function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Keyframe {
    /// Time offset from the start of the timeline.
    pub time: DurationSecs,

    /// The property value at this keyframe.
    pub value: PropertyValue,

    /// Easing function to interpolate FROM this keyframe TO the next.
    #[serde(default)]
    pub easing: EasingFunction,
}

/// Properties that can be animated via keyframe tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnimatableProperty {
    PositionX,
    PositionY,
    Position,
    Rotation,
    ScaleX,
    ScaleY,
    Scale,
    Opacity,
    FillColor,
    StrokeColor,
    StrokeWidth,
}

/// A typed property value for keyframes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PropertyValue {
    Scalar(f64),
    Vec2(Vec2),
    Color(Color),
}

// ---------------------------------------------------------------------------
// Easing functions
// ---------------------------------------------------------------------------

/// Easing functions for interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    /// Custom cubic bezier: (x1, y1, x2, y2).
    CubicBezier,
    /// Discrete step — no interpolation, jump to value.
    Step,
}

impl Default for EasingFunction {
    fn default() -> Self {
        Self::EaseInOut
    }
}

// ---------------------------------------------------------------------------
// Animation Events (discrete transitions)
// ---------------------------------------------------------------------------

/// A discrete animation event — a Manim-style transition block.
///
/// Events have a start time, duration, type, and target nodes.
/// They represent actions like FadeIn, Transform, Write, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnimationEvent {
    /// Unique identifier for this event.
    pub id: EventId,

    /// When this event starts on the timeline.
    pub start_time: DurationSecs,

    /// Duration of the event.
    pub duration: DurationSecs,

    /// What kind of animation to perform.
    pub event_type: EventType,

    /// Nodes targeted by this event.
    pub target_nodes: Vec<NodeId>,

    /// Additional parameters for the event.
    #[serde(default)]
    pub params: EventParams,

    /// Optional human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Types of discrete animation events.
/// These map closely to Manim's animation classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // --- Appearance ---
    FadeIn,
    FadeOut,
    GrowFromCenter,
    ShrinkToCenter,

    // --- Drawing ---
    Create,
    Uncreate,
    Write,
    Unwrite,
    DrawBorderThenFill,

    // --- Transform ---
    Transform,
    MorphTo,
    ReplacementTransform,

    // --- Movement ---
    MoveTo,
    MoveAlongPath,
    Rotate,
    ScaleTo,

    // --- Highlighting ---
    Indicate,
    Flash,
    Circumscribe,
    Wiggle,

    // --- Camera ---
    CameraMove,
    CameraZoom,

    // --- Grouping ---
    AnimationGroup,
    Succession,

    // --- Custom ---
    Custom,
}

/// Additional parameters for animation events.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventParams {
    /// Target position for move operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_position: Option<Vec2>,

    /// Target rotation for rotate operations (radians).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_rotation: Option<f64>,

    /// Target scale for scale operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_scale: Option<Vec2>,

    /// Target node for transform/morph operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<NodeId>,

    /// Path points for MoveAlongPath.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_points: Vec<Vec2>,

    /// Easing function for this event.
    #[serde(default)]
    pub easing: EasingFunction,

    /// For AnimationGroup: child event IDs to play simultaneously.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub group_events: Vec<EventId>,

    /// For Succession: child event IDs to play sequentially.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence_events: Vec<EventId>,

    /// Custom parameters (key-value pairs for extension).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_default() {
        let tl = Timeline::default();
        assert_eq!(tl.duration, DurationSecs::ZERO);
        assert!(tl.tracks.is_empty());
        assert!(tl.events.is_empty());
    }

    #[test]
    fn test_keyframe_track_serde() {
        let track = KeyframeTrack {
            target_node: NodeId::new(),
            property: AnimatableProperty::Opacity,
            keyframes: vec![
                Keyframe {
                    time: DurationSecs::new(0.0),
                    value: PropertyValue::Scalar(0.0),
                    easing: EasingFunction::EaseIn,
                },
                Keyframe {
                    time: DurationSecs::new(1.0),
                    value: PropertyValue::Scalar(1.0),
                    easing: EasingFunction::Linear,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&track).unwrap();
        let deserialized: KeyframeTrack = serde_json::from_str(&json).unwrap();
        assert_eq!(track, deserialized);
    }

    #[test]
    fn test_animation_event_serde() {
        let event = AnimationEvent {
            id: EventId::new(),
            start_time: DurationSecs::new(0.0),
            duration: DurationSecs::new(1.0),
            event_type: EventType::FadeIn,
            target_nodes: vec![NodeId::new()],
            params: EventParams::default(),
            label: Some("Fade in title".to_string()),
        };

        let json = serde_json::to_string_pretty(&event).unwrap();
        let deserialized: AnimationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_sorted_events() {
        let mut tl = Timeline::with_duration(DurationSecs::new(5.0));
        tl.events.push(AnimationEvent {
            id: EventId::new(),
            start_time: DurationSecs::new(2.0),
            duration: DurationSecs::new(1.0),
            event_type: EventType::FadeOut,
            target_nodes: vec![],
            params: EventParams::default(),
            label: None,
        });
        tl.events.push(AnimationEvent {
            id: EventId::new(),
            start_time: DurationSecs::new(0.5),
            duration: DurationSecs::new(1.0),
            event_type: EventType::FadeIn,
            target_nodes: vec![],
            params: EventParams::default(),
            label: None,
        });

        let sorted = tl.sorted_events();
        assert!(sorted[0].start_time.0 < sorted[1].start_time.0);
        assert_eq!(sorted[0].event_type, EventType::FadeIn);
        assert_eq!(sorted[1].event_type, EventType::FadeOut);
    }

    #[test]
    fn test_property_value_variants() {
        let scalar = PropertyValue::Scalar(42.0);
        let vec = PropertyValue::Vec2(Vec2::new(1.0, 2.0));
        let color = PropertyValue::Color(Color::rgb(1.0, 0.0, 0.0));

        // Roundtrip each variant
        for val in [&scalar, &vec, &color] {
            let json = serde_json::to_string(val).unwrap();
            let back: PropertyValue = serde_json::from_str(&json).unwrap();
            assert_eq!(*val, back);
        }
    }
}
