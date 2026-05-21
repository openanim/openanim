//! Scene definition — a single animation scene within a project.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::timeline::Timeline;
use crate::types::{DurationSecs, NodeId, SceneId};

/// A single animation scene.
///
/// Each scene has its own node tree (rooted at `root_node`) and timeline.
/// A project contains one or more scenes that are rendered sequentially
/// or composed together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Scene {
    /// Unique scene identifier.
    pub id: SceneId,

    /// Human-readable scene name.
    pub name: String,

    /// The root node of this scene's node tree.
    pub root_node: NodeId,

    /// All nodes in this scene, stored flat.
    /// The hierarchy is defined by each node's `parent`/`children` fields.
    pub nodes: Vec<crate::node::Node>,

    /// The animation timeline for this scene.
    #[serde(default)]
    pub timeline: Timeline,

    /// Total scene duration (derived from timeline, but stored for convenience).
    #[serde(default)]
    pub duration: DurationSecs,

    /// Scene metadata.
    #[serde(default)]
    pub metadata: SceneMetadata,
}

impl Scene {
    /// Create a new scene with a root group node.
    pub fn new(name: impl Into<String>) -> Self {
        let root = crate::node::Node::named("root", crate::node::NodeType::Group);
        let root_id = root.id;

        Self {
            id: SceneId::new(),
            name: name.into(),
            root_node: root_id,
            nodes: vec![root],
            timeline: Timeline::default(),
            duration: DurationSecs::ZERO,
            metadata: SceneMetadata::default(),
        }
    }

    /// Find a node by ID.
    pub fn find_node(&self, id: NodeId) -> Option<&crate::node::Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find a node by name.
    pub fn find_node_by_name(&self, name: &str) -> Option<&crate::node::Node> {
        self.nodes.iter().find(|n| n.name.as_deref() == Some(name))
    }

    /// Get the number of nodes in this scene.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Metadata for a scene.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SceneMetadata {
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Tags for categorization and search.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Background color for this scene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<crate::types::Color>,

    /// Per-scene render settings override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_override: Option<SceneRenderOverride>,
}

/// Per-scene render settings that override the project-level defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SceneRenderOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<(u32, u32)>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_new() {
        let scene = Scene::new("Intro");
        assert_eq!(scene.name, "Intro");
        assert_eq!(scene.node_count(), 1); // just the root
        assert!(scene.find_node(scene.root_node).is_some());
    }

    #[test]
    fn test_scene_find_by_name() {
        let scene = Scene::new("Test");
        let root = scene.find_node_by_name("root");
        assert!(root.is_some());
        assert_eq!(root.unwrap().id, scene.root_node);
    }

    #[test]
    fn test_scene_serde_roundtrip() {
        let scene = Scene::new("Demo Scene");
        let json = serde_json::to_string_pretty(&scene).unwrap();
        let deserialized: Scene = serde_json::from_str(&json).unwrap();
        assert_eq!(scene, deserialized);
    }
}
