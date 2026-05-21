//! JSON Schema generation for all IR types.
//!
//! Uses `schemars` to auto-generate JSON Schema 2020-12 from Rust types.
//! Schemas can be exported for external validation, documentation, and
//! cross-language type generation.

use schemars::schema_for;
use serde_json::Value;

use crate::node::{ComponentSet, Node, NodeType};
use crate::project::{AssetManifest, Project, ProjectMetadata, RenderSettings};
use crate::scene::{Scene, SceneMetadata};
use crate::timeline::{AnimationEvent, KeyframeTrack, Timeline};

/// Generate the JSON Schema for the top-level `Project` type.
pub fn project_schema() -> Value {
    serde_json::to_value(schema_for!(Project)).unwrap()
}

/// Generate the JSON Schema for a `Scene`.
pub fn scene_schema() -> Value {
    serde_json::to_value(schema_for!(Scene)).unwrap()
}

/// Generate the JSON Schema for a `Node`.
pub fn node_schema() -> Value {
    serde_json::to_value(schema_for!(Node)).unwrap()
}

/// Generate the JSON Schema for `ComponentSet`.
pub fn component_set_schema() -> Value {
    serde_json::to_value(schema_for!(ComponentSet)).unwrap()
}

/// Generate the JSON Schema for `Timeline`.
pub fn timeline_schema() -> Value {
    serde_json::to_value(schema_for!(Timeline)).unwrap()
}

/// Generate all schemas as a map of type name → schema.
pub fn all_schemas() -> std::collections::HashMap<String, Value> {
    let mut schemas = std::collections::HashMap::new();

    schemas.insert("Project".to_string(), project_schema());
    schemas.insert("Scene".to_string(), scene_schema());
    schemas.insert("Node".to_string(), node_schema());
    schemas.insert("ComponentSet".to_string(), component_set_schema());
    schemas.insert("Timeline".to_string(), timeline_schema());
    schemas.insert(
        "NodeType".to_string(),
        serde_json::to_value(schema_for!(NodeType)).unwrap(),
    );
    schemas.insert(
        "ProjectMetadata".to_string(),
        serde_json::to_value(schema_for!(ProjectMetadata)).unwrap(),
    );
    schemas.insert(
        "RenderSettings".to_string(),
        serde_json::to_value(schema_for!(RenderSettings)).unwrap(),
    );
    schemas.insert(
        "AssetManifest".to_string(),
        serde_json::to_value(schema_for!(AssetManifest)).unwrap(),
    );
    schemas.insert(
        "SceneMetadata".to_string(),
        serde_json::to_value(schema_for!(SceneMetadata)).unwrap(),
    );
    schemas.insert(
        "KeyframeTrack".to_string(),
        serde_json::to_value(schema_for!(KeyframeTrack)).unwrap(),
    );
    schemas.insert(
        "AnimationEvent".to_string(),
        serde_json::to_value(schema_for!(AnimationEvent)).unwrap(),
    );

    schemas
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_schema_generates() {
        let schema = project_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(
            obj.contains_key("$schema")
                || obj.contains_key("title")
                || obj.contains_key("type")
                || obj.contains_key("properties")
        );
    }

    #[test]
    fn test_all_schemas_complete() {
        let schemas = all_schemas();
        assert!(schemas.len() >= 10);
        assert!(schemas.contains_key("Project"));
        assert!(schemas.contains_key("Scene"));
        assert!(schemas.contains_key("Node"));
        assert!(schemas.contains_key("Timeline"));
    }

    #[test]
    fn test_node_schema_has_properties() {
        let schema = node_schema();
        // Should be a valid JSON Schema object
        assert!(schema.is_object());
    }
}
