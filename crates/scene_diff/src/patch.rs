//! Apply diffs to scenes (patching).

use thiserror::Error;

use scene_ir::scene::Scene;
use scene_ir::types::NodeId;

use crate::diff::{DiffOp, SceneDiff};

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("Node {0} not found in scene")]
    NodeNotFound(NodeId),
    #[error("Parent node {0} not found")]
    ParentNotFound(NodeId),
}

/// Apply a diff to a scene, mutating it in place.
pub fn apply_diff(scene: &mut Scene, diff: &SceneDiff) -> Result<(), PatchError> {
    for op in &diff.ops {
        match op {
            DiffOp::AddNode { parent_id, node } => {
                if let Some(pid) = parent_id {
                    // Add to parent's children.
                    let parent = scene
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == *pid)
                        .ok_or(PatchError::ParentNotFound(*pid))?;
                    parent.children.push(node.id);
                }
                scene.nodes.push(node.clone());
            }
            DiffOp::RemoveNode { node_id } => {
                // Remove from any parent's children list.
                let parent_id = scene
                    .nodes
                    .iter()
                    .find(|n| n.id == *node_id)
                    .and_then(|n| n.parent);
                if let Some(pid) = parent_id {
                    if let Some(parent) = scene.nodes.iter_mut().find(|n| n.id == pid) {
                        parent.children.retain(|&c| c != *node_id);
                    }
                }
                scene.nodes.retain(|n| n.id != *node_id);
            }
            DiffOp::MoveNode {
                node_id,
                old_parent,
                new_parent,
            } => {
                // Remove from old parent.
                if let Some(op) = old_parent {
                    if let Some(parent) = scene.nodes.iter_mut().find(|n| n.id == *op) {
                        parent.children.retain(|&c| c != *node_id);
                    }
                }
                // Add to new parent.
                if let Some(np) = new_parent {
                    let parent = scene
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == *np)
                        .ok_or(PatchError::ParentNotFound(*np))?;
                    parent.children.push(*node_id);
                }
                // Update node's parent field.
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.parent = *new_parent;
            }
            DiffOp::UpdateName { node_id, new, .. } => {
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.name = new.clone();
            }
            DiffOp::UpdateTransform { node_id, new, .. } => {
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.components.transform = new.clone();
            }
            DiffOp::UpdateStyle { node_id, new, .. } => {
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.components.style = new.clone();
            }
            DiffOp::UpdateShape { node_id, new, .. } => {
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.components.shape = new.clone();
            }
            DiffOp::UpdateText { node_id, new, .. } => {
                let node = scene
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                node.components.text = new.clone();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::diff_scenes;
    use scene_ir::components::Transform;
    use scene_ir::node::{Node, NodeType};

    #[test]
    fn test_apply_diff_roundtrip() {
        let scene1 = Scene::new("test");
        let mut scene2 = scene1.clone();
        scene2.nodes[0].components.transform = Some(Transform::at(42.0, 99.0));

        let diff = diff_scenes(&scene1, &scene2);
        let mut patched = scene1.clone();
        apply_diff(&mut patched, &diff).unwrap();

        assert_eq!(
            patched.nodes[0].components.transform,
            scene2.nodes[0].components.transform
        );
    }

    #[test]
    fn test_apply_add_node() {
        let scene1 = Scene::new("test");
        let root_id = scene1.root_node;
        let mut scene2 = scene1.clone();
        let mut new_node = Node::named("added", NodeType::Shape);
        new_node.parent = Some(root_id);
        scene2.nodes[0].children.push(new_node.id);
        scene2.nodes.push(new_node);

        let diff = diff_scenes(&scene1, &scene2);
        let mut patched = scene1.clone();
        apply_diff(&mut patched, &diff).unwrap();
        assert_eq!(patched.nodes.len(), 2);
    }

    #[test]
    fn test_apply_remove_node() {
        let mut scene1 = Scene::new("test");
        let root_id = scene1.root_node;
        let mut extra = Node::named("extra", NodeType::Shape);
        extra.parent = Some(root_id);
        scene1.nodes[0].children.push(extra.id);
        scene1.nodes.push(extra);

        let scene2 = Scene {
            nodes: vec![scene1.nodes[0].clone()],
            ..scene1.clone()
        };
        // Fix children list in scene2.
        let mut scene2 = scene2;
        scene2.nodes[0].children.clear();

        let diff = diff_scenes(&scene1, &scene2);
        let mut patched = scene1.clone();
        apply_diff(&mut patched, &diff).unwrap();
        assert_eq!(patched.nodes.len(), 1);
    }
}
