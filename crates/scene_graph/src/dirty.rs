//! Dirty tracking for incremental rendering.

use std::collections::HashMap;

use bitflags::bitflags;
use scene_ir::types::NodeId;

use crate::graph::SceneGraph;

bitflags! {
    /// Flags indicating which aspects of a node have changed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DirtyFlags: u32 {
        const TRANSFORM = 0x01;
        const STYLE     = 0x02;
        const CONTENT   = 0x04;
        const CHILDREN  = 0x08;
        const TIMELINE  = 0x10;
        const ALL       = 0x1F;
    }
}

/// Tracks which nodes have changed and need re-rendering.
#[derive(Debug, Default)]
pub struct DirtyTracker {
    dirty: HashMap<NodeId, DirtyFlags>,
}

impl DirtyTracker {
    pub fn new() -> Self {
        Self {
            dirty: HashMap::new(),
        }
    }

    /// Mark a node with specific dirty flags.
    pub fn mark(&mut self, id: NodeId, flags: DirtyFlags) {
        self.dirty
            .entry(id)
            .and_modify(|f| *f |= flags)
            .or_insert(flags);
    }

    /// Clear dirty flags for a specific node.
    pub fn clear(&mut self, id: NodeId) {
        self.dirty.remove(&id);
    }

    /// Clear all dirty flags.
    pub fn clear_all(&mut self) {
        self.dirty.clear();
    }

    /// Check if a node is dirty.
    pub fn is_dirty(&self, id: NodeId) -> bool {
        self.dirty.contains_key(&id)
    }

    /// Get the dirty flags for a node.
    pub fn flags(&self, id: NodeId) -> Option<DirtyFlags> {
        self.dirty.get(&id).copied()
    }

    /// Get all dirty nodes and their flags.
    pub fn dirty_nodes(&self) -> Vec<(NodeId, DirtyFlags)> {
        self.dirty.iter().map(|(&id, &flags)| (id, flags)).collect()
    }

    /// Number of dirty nodes.
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    /// When a node's transform changes, propagate TRANSFORM dirty to all descendants.
    pub fn propagate_transform(&mut self, graph: &SceneGraph, id: NodeId) {
        self.mark(id, DirtyFlags::TRANSFORM);
        for desc_id in graph.descendants(id) {
            self.mark(desc_id, DirtyFlags::TRANSFORM);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::node::{Node, NodeType};

    #[test]
    fn test_mark_and_check() {
        let mut tracker = DirtyTracker::new();
        let id = NodeId::new();

        assert!(!tracker.is_dirty(id));
        tracker.mark(id, DirtyFlags::TRANSFORM);
        assert!(tracker.is_dirty(id));
        assert_eq!(tracker.flags(id), Some(DirtyFlags::TRANSFORM));
    }

    #[test]
    fn test_mark_combines_flags() {
        let mut tracker = DirtyTracker::new();
        let id = NodeId::new();

        tracker.mark(id, DirtyFlags::TRANSFORM);
        tracker.mark(id, DirtyFlags::STYLE);

        let flags = tracker.flags(id).unwrap();
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(flags.contains(DirtyFlags::STYLE));
    }

    #[test]
    fn test_clear() {
        let mut tracker = DirtyTracker::new();
        let id = NodeId::new();

        tracker.mark(id, DirtyFlags::ALL);
        assert!(tracker.is_dirty(id));

        tracker.clear(id);
        assert!(!tracker.is_dirty(id));
    }

    #[test]
    fn test_clear_all() {
        let mut tracker = DirtyTracker::new();
        tracker.mark(NodeId::new(), DirtyFlags::TRANSFORM);
        tracker.mark(NodeId::new(), DirtyFlags::STYLE);
        assert_eq!(tracker.dirty_count(), 2);

        tracker.clear_all();
        assert_eq!(tracker.dirty_count(), 0);
    }

    #[test]
    fn test_propagate_transform() {
        let mut graph = SceneGraph::new();
        let root = Node::named("root", NodeType::Group);
        let root_id = root.id;
        graph.add_node(root, None);

        let child = Node::named("child", NodeType::Shape);
        let child_id = child.id;
        graph.add_node(child, Some(root_id));

        let grandchild = Node::named("gc", NodeType::Shape);
        let gc_id = grandchild.id;
        graph.add_node(grandchild, Some(child_id));

        let mut tracker = DirtyTracker::new();
        tracker.propagate_transform(&graph, root_id);

        assert!(tracker.is_dirty(root_id));
        assert!(tracker.is_dirty(child_id));
        assert!(tracker.is_dirty(gc_id));
        assert_eq!(tracker.dirty_count(), 3);
    }
}
