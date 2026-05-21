//! Scene graph node definition with component set.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::components::{
    CodeBlock, Diagram, ImageContent, LayoutConstraint, MathExpression, Shape, Style, TextContent,
    Transform,
};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// Node type
// ---------------------------------------------------------------------------

/// The semantic type of a scene graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Invisible grouping node (only has transform + children).
    Group,
    /// Geometric shape (rectangle, circle, line, polygon, path, etc.).
    Shape,
    /// Text content with typography.
    Text,
    /// LaTeX mathematical expression.
    Math,
    /// Image from an external asset.
    Image,
    /// Source code block with syntax highlighting.
    Code,
    /// Diagram (Mermaid, PlantUML, etc.).
    Diagram,
    /// Custom/extension node type.
    Custom,
}

// ---------------------------------------------------------------------------
// Component set
// ---------------------------------------------------------------------------

/// The set of all components attached to a node.
///
/// This is the ECS "component bag" — each field is optional, and nodes gain
/// behavior by which components are present. A node with `transform + style + shape`
/// is a visible shape; a node with only `transform` is an invisible group.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ComponentSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextContent>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math: Option<MathExpression>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageContent>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeBlock>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagram: Option<Diagram>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<LayoutConstraint>,
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A single node in the scene graph.
///
/// Nodes form a tree via `parent` / `children` relationships. Each node carries
/// an optional set of components that define its behavior and appearance.
///
/// Identity: `id` (UUID v7) is stable across edits. Content-hashes are computed
/// separately by the `hasher` crate for cache keying.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    /// Stable unique identifier (UUID v7, never changes).
    pub id: NodeId,

    /// Human-readable name for debugging and querying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Semantic type of this node.
    pub node_type: NodeType,

    /// Parent node ID. `None` for root nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeId>,

    /// Ordered list of child node IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeId>,

    /// Attached components.
    #[serde(default)]
    pub components: ComponentSet,
}

impl Node {
    /// Create a new node with the given type and default components.
    pub fn new(node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(),
            name: None,
            node_type,
            parent: None,
            children: Vec::new(),
            components: ComponentSet::default(),
        }
    }

    /// Create a new node with a specific ID (for deserialization/testing).
    pub fn with_id(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            name: None,
            node_type,
            parent: None,
            children: Vec::new(),
            components: ComponentSet::default(),
        }
    }

    /// Create a new named node.
    pub fn named(name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(),
            name: Some(name.into()),
            node_type,
            parent: None,
            children: Vec::new(),
            components: ComponentSet::default(),
        }
    }

    /// Create a group node (invisible container).
    pub fn group() -> Self {
        Self::new(NodeType::Group)
    }

    /// Check if this node has any visual components.
    pub fn is_renderable(&self) -> bool {
        self.components.shape.is_some()
            || self.components.text.is_some()
            || self.components.math.is_some()
            || self.components.image.is_some()
            || self.components.code.is_some()
            || self.components.diagram.is_some()
    }

    /// Check if this node is a leaf (no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this node is a root (no parent).
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{ShapeKind, Style};
    use crate::types::Color;

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeType::Shape);
        assert!(node.is_root());
        assert!(node.is_leaf());
        assert!(!node.is_renderable()); // no shape component yet
    }

    #[test]
    fn test_node_named() {
        let node = Node::named("title", NodeType::Text);
        assert_eq!(node.name, Some("title".to_string()));
        assert_eq!(node.node_type, NodeType::Text);
    }

    #[test]
    fn test_node_renderable() {
        let mut node = Node::new(NodeType::Shape);
        assert!(!node.is_renderable());

        node.components.shape = Some(Shape {
            kind: ShapeKind::Circle { radius: 50.0 },
        });
        assert!(node.is_renderable());
    }

    #[test]
    fn test_node_serde_roundtrip() {
        let mut node = Node::named("my_circle", NodeType::Shape);
        node.components.transform = Some(Transform::at(100.0, 200.0));
        node.components.style = Some(Style::fill(Color::rgb(0.2, 0.5, 0.8)));
        node.components.shape = Some(Shape {
            kind: ShapeKind::Circle { radius: 50.0 },
        });

        let json = serde_json::to_string_pretty(&node).unwrap();
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(node, deserialized);
    }

    #[test]
    fn test_node_minimal_json() {
        // A group node with no components should produce minimal JSON
        let node = Node::new(NodeType::Group);
        let json = serde_json::to_value(&node).unwrap();
        // Should not have empty optional fields
        assert!(!json.as_object().unwrap().contains_key("name"));
        assert!(!json.as_object().unwrap().contains_key("parent"));
    }

    #[test]
    fn test_component_set_default_empty() {
        let cs = ComponentSet::default();
        assert!(cs.transform.is_none());
        assert!(cs.style.is_none());
        assert!(cs.shape.is_none());
        assert!(cs.text.is_none());
        assert!(cs.constraints.is_empty());
    }
}
