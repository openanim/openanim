//! Render plan: compiled instructions for a renderer.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use scene_ir::types::SceneId;

/// A compiled render plan — the instructions a renderer will execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPlan {
    /// Unique plan identifier.
    pub id: Uuid,
    /// Name of the provider that compiled this plan.
    pub provider_name: String,
    /// Scene this plan targets.
    pub scene_id: SceneId,
    /// Ordered list of commands to execute.
    pub commands: Vec<RenderCommand>,
    /// Estimated execution duration (if known).
    pub estimated_duration: Option<Duration>,
}

impl RenderPlan {
    pub fn new(provider_name: impl Into<String>, scene_id: SceneId) -> Self {
        Self {
            id: Uuid::now_v7(),
            provider_name: provider_name.into(),
            scene_id,
            commands: Vec::new(),
            estimated_duration: None,
        }
    }

    pub fn add_command(&mut self, cmd: RenderCommand) {
        self.commands.push(cmd);
    }
}

/// Individual commands within a render plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RenderCommand {
    /// Generate source code for the renderer.
    GenerateCode {
        code: String,
        language: String,
        output_path: String,
    },
    /// Execute an external process.
    ExecuteProcess {
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default = "default_timeout")]
        timeout_secs: u64,
        working_dir: Option<String>,
    },
    /// Copy an asset file.
    CopyAsset {
        source: String,
        destination: String,
    },
    /// Compose multiple outputs (e.g., concat video segments).
    Compose {
        inputs: Vec<String>,
        output: String,
        strategy: String,
    },
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plan_new() {
        let plan = RenderPlan::new("manim", SceneId::new());
        assert_eq!(plan.provider_name, "manim");
        assert!(plan.commands.is_empty());
    }

    #[test]
    fn test_render_plan_serde() {
        let mut plan = RenderPlan::new("remotion", SceneId::new());
        plan.add_command(RenderCommand::GenerateCode {
            code: "print('hello')".to_string(),
            language: "python".to_string(),
            output_path: "/tmp/scene.py".to_string(),
        });
        plan.add_command(RenderCommand::ExecuteProcess {
            command: "manim".to_string(),
            args: vec!["render".to_string(), "scene.py".to_string()],
            env: HashMap::new(),
            timeout_secs: 120,
            working_dir: None,
        });

        let json = serde_json::to_string_pretty(&plan).unwrap();
        let deserialized: RenderPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.commands.len(), 2);
    }
}
