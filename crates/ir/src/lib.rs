//! Canonical, renderer-agnostic Scene IR.
//!
//! This is intentionally small to start: we want a stable, versioned envelope that
//! can evolve without breaking determinism.

use serde::{Deserialize, Serialize};

pub const IR_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub ir_version: u32,
    pub scenes: Vec<Scene>,
}

impl Project {
    pub fn new() -> Self {
        Self {
            ir_version: IR_VERSION,
            scenes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub timeline: Timeline,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    /// Semantic steps (animations, transitions, overlays).
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    /// Placeholder: a logical unit that a provider can compile/render.
    Note { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_roundtrips_json() {
        let mut p = Project::new();
        p.scenes.push(Scene {
            id: "scene-1".to_string(),
            timeline: Timeline {
                steps: vec![Step::Note {
                    text: "hello".to_string(),
                }],
            },
        });

        let json = serde_json::to_string_pretty(&p).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        assert_eq!(back.ir_version, IR_VERSION);
    }
}

