//! IR validation: referential integrity, hierarchy acyclicity,
//! timeline consistency, and structural correctness.

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::node::Node;
use crate::project::Project;
use crate::scene::Scene;
use crate::types::NodeId;

/// Validation errors for the scene IR.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Node {node_id} references non-existent parent {parent_id}")]
    OrphanedParentRef { node_id: NodeId, parent_id: NodeId },

    #[error("Node {node_id} lists child {child_id} which does not exist")]
    MissingChild { node_id: NodeId, child_id: NodeId },

    #[error(
        "Node {child_id} has parent {expected_parent} but is listed as child of {actual_parent}"
    )]
    InconsistentParentChild {
        child_id: NodeId,
        expected_parent: NodeId,
        actual_parent: NodeId,
    },

    #[error("Cycle detected in node hierarchy involving node {node_id}")]
    CycleDetected { node_id: NodeId },

    #[error("Root node {root_id} not found in scene nodes")]
    MissingRootNode { root_id: NodeId },

    #[error("Root node {root_id} has a parent set (root nodes must have parent = None)")]
    RootHasParent { root_id: NodeId },

    #[error("Timeline event references non-existent node {node_id}")]
    EventTargetMissing { node_id: NodeId },

    #[error("Timeline track references non-existent node {node_id}")]
    TrackTargetMissing { node_id: NodeId },

    #[error("Duplicate node ID {node_id} in scene")]
    DuplicateNodeId { node_id: NodeId },

    #[error("Scene has no nodes")]
    EmptyScene,

    #[error("Node {node_id} has multiple parents")]
    MultipleParents { node_id: NodeId },
}

/// Validate an entire project.
pub fn validate_project(project: &Project) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for scene in &project.scenes {
        errors.extend(validate_scene(scene));
    }

    errors
}

/// Validate a single scene.
pub fn validate_scene(scene: &Scene) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check for empty scene.
    if scene.nodes.is_empty() {
        errors.push(ValidationError::EmptyScene);
        return errors;
    }

    // Build node lookup.
    let node_map: HashMap<NodeId, &Node> = scene.nodes.iter().map(|n| (n.id, n)).collect();

    // Check for duplicate IDs.
    {
        let mut seen = HashSet::new();
        for node in &scene.nodes {
            if !seen.insert(node.id) {
                errors.push(ValidationError::DuplicateNodeId { node_id: node.id });
            }
        }
    }

    // Check root node exists.
    if !node_map.contains_key(&scene.root_node) {
        errors.push(ValidationError::MissingRootNode {
            root_id: scene.root_node,
        });
    } else {
        let root = node_map[&scene.root_node];
        if root.parent.is_some() {
            errors.push(ValidationError::RootHasParent {
                root_id: scene.root_node,
            });
        }
    }

    // Validate each node.
    for node in &scene.nodes {
        // Check parent exists.
        if let Some(parent_id) = node.parent {
            if !node_map.contains_key(&parent_id) {
                errors.push(ValidationError::OrphanedParentRef {
                    node_id: node.id,
                    parent_id,
                });
            }
        }

        // Check children exist and have correct parent.
        for &child_id in &node.children {
            match node_map.get(&child_id) {
                None => {
                    errors.push(ValidationError::MissingChild {
                        node_id: node.id,
                        child_id,
                    });
                }
                Some(child) => {
                    if child.parent != Some(node.id) {
                        errors.push(ValidationError::InconsistentParentChild {
                            child_id,
                            expected_parent: child.parent.unwrap_or(NodeId::nil()),
                            actual_parent: node.id,
                        });
                    }
                }
            }
        }
    }

    // Check for cycles via DFS.
    {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for node in &scene.nodes {
            if !visited.contains(&node.id) {
                if let Some(cycle_node) = detect_cycle(node.id, &node_map, &mut visited, &mut stack)
                {
                    errors.push(ValidationError::CycleDetected {
                        node_id: cycle_node,
                    });
                }
            }
        }
    }

    // Validate timeline references.
    for track in &scene.timeline.tracks {
        if !node_map.contains_key(&track.target_node) {
            errors.push(ValidationError::TrackTargetMissing {
                node_id: track.target_node,
            });
        }
    }

    for event in &scene.timeline.events {
        for &target in &event.target_nodes {
            if !node_map.contains_key(&target) {
                errors.push(ValidationError::EventTargetMissing { node_id: target });
            }
        }
    }

    errors
}

/// DFS cycle detection. Returns Some(node_id) if a cycle is found.
fn detect_cycle(
    node_id: NodeId,
    node_map: &HashMap<NodeId, &Node>,
    visited: &mut HashSet<NodeId>,
    stack: &mut HashSet<NodeId>,
) -> Option<NodeId> {
    visited.insert(node_id);
    stack.insert(node_id);

    if let Some(node) = node_map.get(&node_id) {
        for &child_id in &node.children {
            if !visited.contains(&child_id) {
                if let Some(cycle) = detect_cycle(child_id, node_map, visited, stack) {
                    return Some(cycle);
                }
            } else if stack.contains(&child_id) {
                return Some(child_id);
            }
        }
    }

    stack.remove(&node_id);
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, NodeType};

    fn make_valid_scene() -> Scene {
        let mut root = Node::named("root", NodeType::Group);
        let child = Node::named("child", NodeType::Shape);

        root.children.push(child.id);
        let mut child = child;
        child.parent = Some(root.id);

        let root_id = root.id;
        Scene {
            id: crate::types::SceneId::new(),
            name: "test".to_string(),
            root_node: root_id,
            nodes: vec![root, child],
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_valid_scene() {
        let scene = make_valid_scene();
        let errors = validate_scene(&scene);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_missing_root() {
        let scene = Scene {
            id: crate::types::SceneId::new(),
            name: "test".to_string(),
            root_node: NodeId::new(), // doesn't exist
            nodes: vec![Node::new(NodeType::Group)],
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        };
        let errors = validate_scene(&scene);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::MissingRootNode { .. }))
        );
    }

    #[test]
    fn test_duplicate_ids() {
        let node = Node::new(NodeType::Group);
        let id = node.id;
        let scene = Scene {
            id: crate::types::SceneId::new(),
            name: "test".to_string(),
            root_node: id,
            nodes: vec![node.clone(), node],
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        };
        let errors = validate_scene(&scene);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateNodeId { .. }))
        );
    }

    #[test]
    fn test_empty_scene() {
        let scene = Scene {
            id: crate::types::SceneId::new(),
            name: "empty".to_string(),
            root_node: NodeId::nil(),
            nodes: vec![],
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        };
        let errors = validate_scene(&scene);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::EmptyScene))
        );
    }

    #[test]
    fn test_orphaned_parent_ref() {
        let mut node = Node::new(NodeType::Shape);
        node.parent = Some(NodeId::new()); // parent doesn't exist
        let id = node.id;

        let scene = Scene {
            id: crate::types::SceneId::new(),
            name: "test".to_string(),
            root_node: id,
            nodes: vec![node],
            timeline: Default::default(),
            duration: Default::default(),
            metadata: Default::default(),
        };
        let errors = validate_scene(&scene);
        // Will have both RootHasParent and OrphanedParentRef
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::OrphanedParentRef { .. }))
        );
    }
}
