//! Fluent builder API for constructing scene graphs programmatically.

use scene_ir::components::{Shape, Style, TextContent, Transform};
use scene_ir::node::{Node, NodeType};
use scene_ir::types::NodeId;

use crate::graph::SceneGraph;

/// A fluent builder for constructing scene graphs.
///
/// # Example
/// ```
/// use scene_graph::SceneGraphBuilder;
/// use scene_ir::node::NodeType;
/// use scene_ir::components::{Transform, Style, Shape, ShapeKind};
/// use scene_ir::types::Color;
///
/// let graph = SceneGraphBuilder::new()
///     .root("scene_root", NodeType::Group)
///     .add_child("circle", NodeType::Shape)
///         .with_transform(Transform::at(100.0, 200.0))
///         .with_style(Style::fill(Color::rgb(0.2, 0.5, 0.8)))
///         .with_shape(Shape { kind: ShapeKind::Circle { radius: 50.0 } })
///     .pop()
///     .add_child("label", NodeType::Text)
///         .with_transform(Transform::at(100.0, 280.0))
///     .pop()
///     .build();
/// ```
pub struct SceneGraphBuilder {
    graph: SceneGraph,
    /// Stack of "current parent" node IDs for the nested builder pattern.
    context_stack: Vec<NodeId>,
}

impl SceneGraphBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            graph: SceneGraph::new(),
            context_stack: Vec::new(),
        }
    }

    /// Set the root node.
    pub fn root(mut self, name: &str, node_type: NodeType) -> Self {
        let node = Node::named(name, node_type);
        let id = self.graph.add_node(node, None);
        self.context_stack.push(id);
        self
    }

    /// Add a child node under the current context.
    pub fn add_child(mut self, name: &str, node_type: NodeType) -> Self {
        let parent = self.context_stack.last().copied();
        let node = Node::named(name, node_type);
        let id = self.graph.add_node(node, parent);
        self.context_stack.push(id);
        self
    }

    /// Set the transform on the current node.
    pub fn with_transform(mut self, transform: Transform) -> Self {
        if let Some(&id) = self.context_stack.last() {
            if let Some(node) = self.graph.get_node_mut(id) {
                node.components.transform = Some(transform);
            }
        }
        self
    }

    /// Set the style on the current node.
    pub fn with_style(mut self, style: Style) -> Self {
        if let Some(&id) = self.context_stack.last() {
            if let Some(node) = self.graph.get_node_mut(id) {
                node.components.style = Some(style);
            }
        }
        self
    }

    /// Set the shape on the current node.
    pub fn with_shape(mut self, shape: Shape) -> Self {
        if let Some(&id) = self.context_stack.last() {
            if let Some(node) = self.graph.get_node_mut(id) {
                node.components.shape = Some(shape);
            }
        }
        self
    }

    /// Set the text content on the current node.
    pub fn with_text(mut self, text: TextContent) -> Self {
        if let Some(&id) = self.context_stack.last() {
            if let Some(node) = self.graph.get_node_mut(id) {
                node.components.text = Some(text);
            }
        }
        self
    }

    /// Go back to the parent context (pop the stack).
    pub fn pop(mut self) -> Self {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
        self
    }

    /// Build the final scene graph.
    pub fn build(self) -> SceneGraph {
        self.graph
    }
}

impl Default for SceneGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::components::ShapeKind;
    use scene_ir::types::Color;

    #[test]
    fn test_builder_basic() {
        let graph = SceneGraphBuilder::new()
            .root("root", NodeType::Group)
            .add_child("child", NodeType::Shape)
            .pop()
            .build();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.root().is_some());
    }

    #[test]
    fn test_builder_with_components() {
        let graph = SceneGraphBuilder::new()
            .root("root", NodeType::Group)
            .add_child("circle", NodeType::Shape)
            .with_transform(Transform::at(100.0, 200.0))
            .with_style(Style::fill(Color::rgb(1.0, 0.0, 0.0)))
            .with_shape(Shape {
                kind: ShapeKind::Circle { radius: 30.0 },
            })
            .pop()
            .build();

        assert_eq!(graph.node_count(), 2);
        let root_id = graph.root().unwrap();
        let children = graph.children(root_id);
        assert_eq!(children.len(), 1);

        let circle = graph.get_node(children[0]).unwrap();
        assert!(circle.components.transform.is_some());
        assert!(circle.components.style.is_some());
        assert!(circle.components.shape.is_some());
    }

    #[test]
    fn test_builder_nested() {
        let graph = SceneGraphBuilder::new()
            .root("root", NodeType::Group)
            .add_child("a", NodeType::Group)
            .add_child("a1", NodeType::Shape)
            .pop()
            .add_child("a2", NodeType::Shape)
            .pop()
            .pop()
            .add_child("b", NodeType::Text)
            .pop()
            .build();

        assert_eq!(graph.node_count(), 5);
        let root_id = graph.root().unwrap();
        let root_children = graph.children(root_id);
        assert_eq!(root_children.len(), 2); // a, b
    }
}
