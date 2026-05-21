//! Structural diffing between two scene states.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use scene_ir::components::{Shape, Style, TextContent, Transform};
use scene_ir::node::Node;
use scene_ir::scene::Scene;
use scene_ir::types::NodeId;

/// A single diff operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum DiffOp {
    AddNode {
        parent_id: Option<NodeId>,
        node: Node,
    },
    RemoveNode {
        node_id: NodeId,
    },
    MoveNode {
        node_id: NodeId,
        old_parent: Option<NodeId>,
        new_parent: Option<NodeId>,
    },
    UpdateName {
        node_id: NodeId,
        old: Option<String>,
        new: Option<String>,
    },
    UpdateTransform {
        node_id: NodeId,
        old: Option<Transform>,
        new: Option<Transform>,
    },
    UpdateStyle {
        node_id: NodeId,
        old: Option<Style>,
        new: Option<Style>,
    },
    UpdateShape {
        node_id: NodeId,
        old: Option<Shape>,
        new: Option<Shape>,
    },
    UpdateText {
        node_id: NodeId,
        old: Option<TextContent>,
        new: Option<TextContent>,
    },
}

/// A collection of diff operations between two scene states.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneDiff {
    pub ops: Vec<DiffOp>,
}

impl SceneDiff {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn op_count(&self) -> usize {
        self.ops.len()
    }
}

/// Compute the structural diff between two scenes.
pub fn diff_scenes(old: &Scene, new: &Scene) -> SceneDiff {
    let mut ops = Vec::new();

    let old_map: HashMap<NodeId, &Node> = old.nodes.iter().map(|n| (n.id, n)).collect();
    let new_map: HashMap<NodeId, &Node> = new.nodes.iter().map(|n| (n.id, n)).collect();

    let old_ids: HashSet<NodeId> = old_map.keys().copied().collect();
    let new_ids: HashSet<NodeId> = new_map.keys().copied().collect();

    // Nodes only in old → removed.
    for &id in old_ids.difference(&new_ids) {
        ops.push(DiffOp::RemoveNode { node_id: id });
    }

    // Nodes only in new → added.
    for &id in new_ids.difference(&old_ids) {
        let node = new_map[&id];
        ops.push(DiffOp::AddNode {
            parent_id: node.parent,
            node: node.clone(),
        });
    }

    // Nodes in both → check for changes.
    for &id in old_ids.intersection(&new_ids) {
        let old_node = old_map[&id];
        let new_node = new_map[&id];

        // Name change.
        if old_node.name != new_node.name {
            ops.push(DiffOp::UpdateName {
                node_id: id,
                old: old_node.name.clone(),
                new: new_node.name.clone(),
            });
        }

        // Parent change (reparent).
        if old_node.parent != new_node.parent {
            ops.push(DiffOp::MoveNode {
                node_id: id,
                old_parent: old_node.parent,
                new_parent: new_node.parent,
            });
        }

        // Transform change.
        if old_node.components.transform != new_node.components.transform {
            ops.push(DiffOp::UpdateTransform {
                node_id: id,
                old: old_node.components.transform.clone(),
                new: new_node.components.transform.clone(),
            });
        }

        // Style change.
        if old_node.components.style != new_node.components.style {
            ops.push(DiffOp::UpdateStyle {
                node_id: id,
                old: old_node.components.style.clone(),
                new: new_node.components.style.clone(),
            });
        }

        // Shape change.
        if old_node.components.shape != new_node.components.shape {
            ops.push(DiffOp::UpdateShape {
                node_id: id,
                old: old_node.components.shape.clone(),
                new: new_node.components.shape.clone(),
            });
        }

        // Text change.
        if old_node.components.text != new_node.components.text {
            ops.push(DiffOp::UpdateText {
                node_id: id,
                old: old_node.components.text.clone(),
                new: new_node.components.text.clone(),
            });
        }
    }

    SceneDiff { ops }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::components::Style;
    use scene_ir::node::NodeType;
    use scene_ir::types::Color;

    #[test]
    fn test_identical_scenes_no_diff() {
        let scene = Scene::new("test");
        let diff = diff_scenes(&scene, &scene);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_added_node() {
        let scene1 = Scene::new("test");
        let mut scene2 = scene1.clone();
        let new_node = Node::named("added", NodeType::Shape);
        scene2.nodes.push(new_node);

        let diff = diff_scenes(&scene1, &scene2);
        assert_eq!(diff.op_count(), 1);
        assert!(matches!(&diff.ops[0], DiffOp::AddNode { .. }));
    }

    #[test]
    fn test_removed_node() {
        let mut scene1 = Scene::new("test");
        scene1.nodes.push(Node::named("extra", NodeType::Shape));
        // scene2 won't have the same root ID, so let's build properly
        let scene2_nodes = vec![scene1.nodes[0].clone()]; // keep only root
        let scene2 = Scene {
            nodes: scene2_nodes,
            ..scene1.clone()
        };

        let diff = diff_scenes(&scene1, &scene2);
        assert!(diff.ops.iter().any(|op| matches!(op, DiffOp::RemoveNode { .. })));
    }

    #[test]
    fn test_updated_transform() {
        let scene1 = Scene::new("test");
        let mut scene2 = scene1.clone();
        scene2.nodes[0].components.transform = Some(Transform::at(100.0, 200.0));

        let diff = diff_scenes(&scene1, &scene2);
        assert!(diff.ops.iter().any(|op| matches!(op, DiffOp::UpdateTransform { .. })));
    }

    #[test]
    fn test_updated_style() {
        let scene1 = Scene::new("test");
        let mut scene2 = scene1.clone();
        scene2.nodes[0].components.style = Some(Style::fill(Color::rgb(1.0, 0.0, 0.0)));

        let diff = diff_scenes(&scene1, &scene2);
        assert!(diff.ops.iter().any(|op| matches!(op, DiffOp::UpdateStyle { .. })));
    }

    #[test]
    fn test_diff_serde_roundtrip() {
        let scene1 = Scene::new("test");
        let mut scene2 = scene1.clone();
        scene2.nodes[0].components.transform = Some(Transform::at(50.0, 50.0));

        let diff = diff_scenes(&scene1, &scene2);
        let json = serde_json::to_string_pretty(&diff).unwrap();
        let deserialized: SceneDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(diff, deserialized);
    }
}
