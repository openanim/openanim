//! Core scene graph data structure with arena-allocated nodes.

use std::collections::{HashMap, VecDeque};

use scene_ir::node::Node;
use scene_ir::scene::Scene;
use scene_ir::types::NodeId;
use slotmap::{DefaultKey, SlotMap};

/// The in-memory scene graph runtime.
///
/// Nodes are arena-allocated via `SlotMap` for cache-friendly storage
/// and O(1) insert/remove. A secondary `HashMap<NodeId, DefaultKey>`
/// provides O(1) lookup by stable UUID.
#[derive(Debug)]
pub struct SceneGraph {
    /// Arena storage for nodes.
    arena: SlotMap<DefaultKey, Node>,
    /// Maps stable NodeId (UUID) → arena slot key.
    id_to_key: HashMap<NodeId, DefaultKey>,
    /// The root node ID.
    root: Option<NodeId>,
}

impl SceneGraph {
    /// Create an empty scene graph.
    pub fn new() -> Self {
        Self {
            arena: SlotMap::new(),
            id_to_key: HashMap::new(),
            root: None,
        }
    }

    /// Build a scene graph from a Scene IR.
    pub fn from_scene(scene: &Scene) -> Self {
        let mut graph = Self::new();
        for node in &scene.nodes {
            graph.insert_node(node.clone());
        }
        graph.root = Some(scene.root_node);
        graph
    }

    /// Export this scene graph back to a Scene IR.
    pub fn to_scene(&self, name: &str) -> Scene {
        let nodes: Vec<Node> = self.arena.values().cloned().collect();
        Scene {
            id: scene_ir::types::SceneId::new(),
            name: name.to_string(),
            root_node: self.root.unwrap_or(NodeId::nil()),
            nodes,
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        }
    }

    /// Insert a node into the graph. Does not modify parent/child links.
    fn insert_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        let key = self.arena.insert(node);
        self.id_to_key.insert(id, key);
        id
    }

    /// Add a new node to the graph. If `parent` is specified, adds it as a child.
    pub fn add_node(&mut self, mut node: Node, parent: Option<NodeId>) -> NodeId {
        node.parent = parent;
        let id = node.id;
        let key = self.arena.insert(node);
        self.id_to_key.insert(id, key);

        // Add to parent's children list.
        if let Some(parent_id) = parent {
            if let Some(parent_key) = self.id_to_key.get(&parent_id).copied() {
                if let Some(parent_node) = self.arena.get_mut(parent_key) {
                    parent_node.children.push(id);
                }
            }
        }

        // If no root is set yet, this becomes the root.
        if self.root.is_none() && parent.is_none() {
            self.root = Some(id);
        }

        id
    }

    /// Remove a node and all its descendants from the graph.
    pub fn remove_node(&mut self, id: NodeId) {
        // Collect descendants first.
        let descendants = self.descendants(id);

        // Remove from parent's children list.
        if let Some(node) = self.get_node(id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent_key) = self.id_to_key.get(&parent_id).copied() {
                    if let Some(parent_node) = self.arena.get_mut(parent_key) {
                        parent_node.children.retain(|&child| child != id);
                    }
                }
            }
        }

        // Remove all descendants.
        for desc_id in descendants {
            if let Some(key) = self.id_to_key.remove(&desc_id) {
                self.arena.remove(key);
            }
        }

        // Remove the node itself.
        if let Some(key) = self.id_to_key.remove(&id) {
            self.arena.remove(key);
        }

        if self.root == Some(id) {
            self.root = None;
        }
    }

    /// Get a reference to a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.id_to_key.get(&id).and_then(|&key| self.arena.get(key))
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.id_to_key
            .get(&id)
            .copied()
            .and_then(move |key| self.arena.get_mut(key))
    }

    /// Move a node to a new parent at a given index in the children list.
    pub fn reparent(&mut self, node_id: NodeId, new_parent: NodeId, index: Option<usize>) {
        // Remove from old parent.
        if let Some(node) = self.get_node(node_id) {
            if let Some(old_parent_id) = node.parent {
                if let Some(old_key) = self.id_to_key.get(&old_parent_id).copied() {
                    if let Some(old_parent) = self.arena.get_mut(old_key) {
                        old_parent.children.retain(|&c| c != node_id);
                    }
                }
            }
        }

        // Set new parent on node.
        if let Some(key) = self.id_to_key.get(&node_id).copied() {
            if let Some(node) = self.arena.get_mut(key) {
                node.parent = Some(new_parent);
            }
        }

        // Add to new parent's children.
        if let Some(parent_key) = self.id_to_key.get(&new_parent).copied() {
            if let Some(parent_node) = self.arena.get_mut(parent_key) {
                match index {
                    Some(i) if i < parent_node.children.len() => {
                        parent_node.children.insert(i, node_id);
                    }
                    _ => {
                        parent_node.children.push(node_id);
                    }
                }
            }
        }
    }

    /// Get the IDs of a node's children.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.get_node(id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    /// Get the ancestor chain from node to root (excluding the node itself).
    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut current = self.get_node(id).and_then(|n| n.parent);
        while let Some(parent_id) = current {
            result.push(parent_id);
            current = self.get_node(parent_id).and_then(|n| n.parent);
        }
        result
    }

    /// Get all descendants of a node (depth-first order, excluding the node itself).
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut stack = self.children(id);
        stack.reverse(); // Process in order
        while let Some(child_id) = stack.pop() {
            result.push(child_id);
            let grandchildren = self.children(child_id);
            for gc in grandchildren.into_iter().rev() {
                stack.push(gc);
            }
        }
        result
    }

    /// Total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }

    /// Get the root node ID.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Iterate over all nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node> {
        self.arena.values()
    }

    /// Depth-first traversal from a given root.
    pub fn dfs<F: FnMut(&Node)>(&self, root: NodeId, mut f: F) {
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get_node(id) {
                f(node);
                // Push children in reverse order so first child is processed first.
                for &child_id in node.children.iter().rev() {
                    stack.push(child_id);
                }
            }
        }
    }

    /// Breadth-first traversal from a given root.
    pub fn bfs<F: FnMut(&Node)>(&self, root: NodeId, mut f: F) {
        let mut queue = VecDeque::new();
        queue.push_back(root);
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get_node(id) {
                f(node);
                for &child_id in &node.children {
                    queue.push_back(child_id);
                }
            }
        }
    }
}

impl Default for SceneGraph {
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
    use scene_ir::components::Transform;
    use scene_ir::node::NodeType;

    fn build_test_graph() -> (SceneGraph, NodeId, NodeId, NodeId) {
        let mut graph = SceneGraph::new();

        let root = Node::named("root", NodeType::Group);
        let root_id = root.id;
        graph.add_node(root, None);

        let child_a = Node::named("child_a", NodeType::Shape);
        let child_a_id = child_a.id;
        graph.add_node(child_a, Some(root_id));

        let child_b = Node::named("child_b", NodeType::Text);
        let child_b_id = child_b.id;
        graph.add_node(child_b, Some(root_id));

        (graph, root_id, child_a_id, child_b_id)
    }

    #[test]
    fn test_new_graph() {
        let graph = SceneGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert!(graph.root().is_none());
    }

    #[test]
    fn test_add_nodes() {
        let (graph, root_id, _, _) = build_test_graph();
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.root(), Some(root_id));
    }

    #[test]
    fn test_parent_child_links() {
        let (graph, root_id, child_a_id, child_b_id) = build_test_graph();

        let children = graph.children(root_id);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0], child_a_id);
        assert_eq!(children[1], child_b_id);

        assert_eq!(graph.get_node(child_a_id).unwrap().parent, Some(root_id));
    }

    #[test]
    fn test_ancestors() {
        let (mut graph, root_id, child_a_id, _) = build_test_graph();

        let grandchild = Node::named("grandchild", NodeType::Shape);
        let gc_id = grandchild.id;
        graph.add_node(grandchild, Some(child_a_id));

        let ancestors = graph.ancestors(gc_id);
        assert_eq!(ancestors, vec![child_a_id, root_id]);
    }

    #[test]
    fn test_descendants() {
        let (mut graph, root_id, child_a_id, child_b_id) = build_test_graph();

        let grandchild = Node::named("gc", NodeType::Shape);
        let gc_id = grandchild.id;
        graph.add_node(grandchild, Some(child_a_id));

        let desc = graph.descendants(root_id);
        assert_eq!(desc.len(), 3);
        assert!(desc.contains(&child_a_id));
        assert!(desc.contains(&child_b_id));
        assert!(desc.contains(&gc_id));
    }

    #[test]
    fn test_remove_node() {
        let (mut graph, root_id, child_a_id, _) = build_test_graph();
        graph.remove_node(child_a_id);
        assert_eq!(graph.node_count(), 2);
        assert!(graph.get_node(child_a_id).is_none());
        assert!(!graph.children(root_id).contains(&child_a_id));
    }

    #[test]
    fn test_reparent() {
        let (mut graph, _, child_a_id, child_b_id) = build_test_graph();
        graph.reparent(child_b_id, child_a_id, None);

        assert_eq!(graph.get_node(child_b_id).unwrap().parent, Some(child_a_id));
        assert!(graph.children(child_a_id).contains(&child_b_id));
    }

    #[test]
    fn test_get_node_mut() {
        let (mut graph, _, child_a_id, _) = build_test_graph();
        let node = graph.get_node_mut(child_a_id).unwrap();
        node.components.transform = Some(Transform::at(42.0, 99.0));

        let node = graph.get_node(child_a_id).unwrap();
        assert_eq!(
            node.components.transform.as_ref().unwrap().position,
            scene_ir::Vec2::new(42.0, 99.0)
        );
    }

    #[test]
    fn test_dfs_order() {
        let (mut graph, root_id, child_a_id, _) = build_test_graph();
        let gc = Node::named("gc", NodeType::Shape);
        graph.add_node(gc, Some(child_a_id));

        let mut visited = Vec::new();
        graph.dfs(root_id, |node| {
            visited.push(node.name.clone());
        });

        assert_eq!(visited[0], Some("root".to_string()));
        assert_eq!(visited[1], Some("child_a".to_string()));
        assert_eq!(visited[2], Some("gc".to_string()));
        assert_eq!(visited[3], Some("child_b".to_string()));
    }

    #[test]
    fn test_bfs_order() {
        let (mut graph, root_id, child_a_id, _) = build_test_graph();
        let gc = Node::named("gc", NodeType::Shape);
        graph.add_node(gc, Some(child_a_id));

        let mut visited = Vec::new();
        graph.bfs(root_id, |node| {
            visited.push(node.name.clone());
        });

        assert_eq!(visited[0], Some("root".to_string()));
        assert_eq!(visited[1], Some("child_a".to_string()));
        assert_eq!(visited[2], Some("child_b".to_string()));
        assert_eq!(visited[3], Some("gc".to_string()));
    }

    #[test]
    fn test_from_scene_roundtrip() {
        let (graph, _, _, _) = build_test_graph();
        let scene = graph.to_scene("test");
        let graph2 = SceneGraph::from_scene(&scene);
        assert_eq!(graph2.node_count(), graph.node_count());
    }
}
