//! # OpenAnim Hasher
//!
//! Deterministic content-hashing for Bazel-style artifact caching.
//! Uses BLAKE3 for fast, collision-resistant hashing.
//!
//! All hash functions are pure and deterministic: same input always
//! produces the same hash output. JSON serialization uses sorted keys
//! for canonical ordering.

use scene_ir::node::Node;
use scene_ir::project::{Project, RenderSettings};
use scene_ir::scene::Scene;

/// Hash raw bytes using BLAKE3, returning a hex string.
pub fn hash_bytes(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Hash any serializable value by converting to canonical JSON first.
pub fn hash_json<T: serde::Serialize>(value: &T) -> String {
    // serde_json produces deterministic output for the same input.
    let json = serde_json::to_string(value).expect("serialization should not fail");
    hash_bytes(json.as_bytes())
}

/// Hash a node's content (ID + type + components, excluding parent/children).
/// This captures "what the node looks like" without structural position.
pub fn hash_node(node: &Node) -> String {
    #[derive(serde::Serialize)]
    struct NodeContent<'a> {
        id: &'a scene_ir::types::NodeId,
        node_type: &'a scene_ir::node::NodeType,
        name: &'a Option<String>,
        components: &'a scene_ir::node::ComponentSet,
    }

    let content = NodeContent {
        id: &node.id,
        node_type: &node.node_type,
        name: &node.name,
        components: &node.components,
    };

    hash_json(&content)
}

/// Hash a node including its children IDs (structural hash).
pub fn hash_node_with_children(node: &Node) -> String {
    #[derive(serde::Serialize)]
    struct NodeStructural<'a> {
        id: &'a scene_ir::types::NodeId,
        node_type: &'a scene_ir::node::NodeType,
        name: &'a Option<String>,
        components: &'a scene_ir::node::ComponentSet,
        parent: &'a Option<scene_ir::types::NodeId>,
        children: &'a Vec<scene_ir::types::NodeId>,
    }

    let structural = NodeStructural {
        id: &node.id,
        node_type: &node.node_type,
        name: &node.name,
        components: &node.components,
        parent: &node.parent,
        children: &node.children,
    };

    hash_json(&structural)
}

/// Hash an entire scene (all nodes + timeline).
pub fn hash_scene(scene: &Scene) -> String {
    hash_json(scene)
}

/// Hash an entire project.
pub fn hash_project(project: &Project) -> String {
    hash_json(project)
}

/// Hash render settings (for cache key composition).
pub fn hash_render_config(settings: &RenderSettings) -> String {
    hash_json(settings)
}

/// Compose a cache key from component hashes.
///
/// Format: `{provider}:{scene_hash}:{config_hash}`
pub fn cache_key(scene_hash: &str, render_config_hash: &str, provider_name: &str) -> String {
    let combined = format!("{provider_name}:{scene_hash}:{render_config_hash}");
    hash_bytes(combined.as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::components::Transform;
    use scene_ir::node::{Node, NodeType};

    #[test]
    fn test_hash_bytes_deterministic() {
        let data = b"hello world";
        let h1 = hash_bytes(data);
        let h2 = hash_bytes(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_bytes_different_input() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_node_deterministic() {
        let node = Node::named("test", NodeType::Shape);
        let h1 = hash_node(&node);
        let h2 = hash_node(&node);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_node_changes_on_modification() {
        let node = Node::named("test", NodeType::Shape);
        let h1 = hash_node(&node);

        let mut modified = node.clone();
        modified.components.transform = Some(Transform::at(100.0, 200.0));
        let h2 = hash_node(&modified);

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_node_ignores_parent_children() {
        let mut node1 = Node::named("test", NodeType::Shape);
        let mut node2 = node1.clone();

        node1.parent = Some(scene_ir::types::NodeId::new());
        node2.parent = Some(scene_ir::types::NodeId::new());

        let h1 = hash_node(&node1);
        let h2 = hash_node(&node2);
        // Content hash excludes parent/children, but includes ID.
        // Since both have same ID (cloned), same hash.
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_node_with_children_includes_structure() {
        let node1 = Node::named("test", NodeType::Group);
        let mut node2 = node1.clone();
        node2.children.push(scene_ir::types::NodeId::new());

        let h1 = hash_node_with_children(&node1);
        let h2 = hash_node_with_children(&node2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_scene_deterministic() {
        let scene = Scene::new("test");
        let h1 = hash_scene(&scene);
        let h2 = hash_scene(&scene);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_cache_key() {
        let key = cache_key("scene123", "config456", "manim");
        assert!(!key.is_empty());

        let key2 = cache_key("scene123", "config456", "manim");
        assert_eq!(key, key2);

        let key3 = cache_key("scene123", "config789", "manim");
        assert_ne!(key, key3);
    }

    #[test]
    fn test_hash_empty_scene() {
        let scene = Scene::new("empty");
        let hash = hash_scene(&scene);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // BLAKE3 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_hash_project_deterministic() {
        let project = Project::new("test");
        let h1 = hash_project(&project);
        let h2 = hash_project(&project);
        assert_eq!(h1, h2);
    }
}
