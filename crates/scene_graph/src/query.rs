//! Query functions for finding nodes in the scene graph.

use scene_ir::node::{ComponentSet, NodeType};
use scene_ir::types::NodeId;

use crate::graph::SceneGraph;

/// Find a node by name.
pub fn find_by_name(graph: &SceneGraph, name: &str) -> Option<NodeId> {
    graph
        .iter_nodes()
        .find(|n| n.name.as_deref() == Some(name))
        .map(|n| n.id)
}

/// Find all nodes of a given type.
pub fn find_by_type(graph: &SceneGraph, node_type: NodeType) -> Vec<NodeId> {
    graph
        .iter_nodes()
        .filter(|n| n.node_type == node_type)
        .map(|n| n.id)
        .collect()
}

/// Find all nodes matching a component predicate.
pub fn find_with_component<F>(graph: &SceneGraph, predicate: F) -> Vec<NodeId>
where
    F: Fn(&ComponentSet) -> bool,
{
    graph
        .iter_nodes()
        .filter(|n| predicate(&n.components))
        .map(|n| n.id)
        .collect()
}

/// Find all renderable nodes (nodes with visual components).
pub fn find_renderable(graph: &SceneGraph) -> Vec<NodeId> {
    graph
        .iter_nodes()
        .filter(|n| n.is_renderable())
        .map(|n| n.id)
        .collect()
}

/// Find all leaf nodes (no children).
pub fn find_leaves(graph: &SceneGraph) -> Vec<NodeId> {
    graph
        .iter_nodes()
        .filter(|n| n.is_leaf())
        .map(|n| n.id)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::components::{Shape, ShapeKind, Style};
    use scene_ir::node::Node;
    use scene_ir::types::Color;

    fn build_query_graph() -> SceneGraph {
        let mut graph = SceneGraph::new();

        let root = Node::named("root", NodeType::Group);
        let root_id = root.id;
        graph.add_node(root, None);

        let mut circle = Node::named("circle", NodeType::Shape);
        circle.components.shape = Some(Shape {
            kind: ShapeKind::Circle { radius: 50.0 },
        });
        circle.components.style = Some(Style::fill(Color::rgb(1.0, 0.0, 0.0)));
        graph.add_node(circle, Some(root_id));

        let mut label = Node::named("label", NodeType::Text);
        label.components.text = Some(scene_ir::components::TextContent {
            content: "Hello".to_string(),
            ..Default::default()
        });
        graph.add_node(label, Some(root_id));

        let group = Node::named("empty_group", NodeType::Group);
        graph.add_node(group, Some(root_id));

        graph
    }

    #[test]
    fn test_find_by_name() {
        let graph = build_query_graph();
        assert!(find_by_name(&graph, "circle").is_some());
        assert!(find_by_name(&graph, "nonexistent").is_none());
    }

    #[test]
    fn test_find_by_type() {
        let graph = build_query_graph();
        let groups = find_by_type(&graph, NodeType::Group);
        assert_eq!(groups.len(), 2); // root + empty_group
    }

    #[test]
    fn test_find_renderable() {
        let graph = build_query_graph();
        let renderable = find_renderable(&graph);
        assert_eq!(renderable.len(), 2); // circle + label
    }

    #[test]
    fn test_find_with_component() {
        let graph = build_query_graph();
        let with_style = find_with_component(&graph, |cs| cs.style.is_some());
        assert_eq!(with_style.len(), 1); // only circle
    }

    #[test]
    fn test_find_leaves() {
        let graph = build_query_graph();
        let leaves = find_leaves(&graph);
        assert_eq!(leaves.len(), 3); // circle, label, empty_group
    }
}
