//! Embeddable OpenAnim engine API.
//!
//! This crate is the integration surface for hosted services and other Rust
//! applications. It keeps file serving, local developer tools, and CLI concerns
//! outside the core engine boundary.

use std::path::PathBuf;

use llm_compiler::LlmCompiler;
use orchestrator::Orchestrator;
use scene_ir::project::{Project, RenderSettings};
use scene_ir::scene::Scene;
use scene_ir::validation::{ValidationError, validate_project, validate_scene};

pub use llm_compiler::LlmProvider;
pub use orchestrator::RenderOptions;
pub use renderer_core::artifact::{ArtifactStatus, RenderArtifact};
pub use scene_ir::{project, scene, validation};

pub struct OpenAnimEngine {
    orchestrator: Orchestrator,
}

impl OpenAnimEngine {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            orchestrator: Orchestrator::new(cache_dir.into()),
        }
    }

    pub fn renderer_names(&self) -> Vec<&str> {
        self.orchestrator.registry.list()
    }

    pub fn validate_project(&self, project: &Project) -> Vec<ValidationError> {
        validate_project(project)
    }

    pub fn validate_scene(&self, scene: &Scene) -> Vec<ValidationError> {
        validate_scene(scene)
    }

    pub async fn render_scene(
        &self,
        scene: &Scene,
        settings: &RenderSettings,
    ) -> anyhow::Result<RenderArtifact> {
        self.orchestrator.render(scene, settings).await
    }

    pub async fn render_scene_with_options(
        &self,
        scene: &Scene,
        settings: &RenderSettings,
        options: &RenderOptions,
    ) -> anyhow::Result<RenderArtifact> {
        self.orchestrator
            .render_with_options(scene, settings, options)
            .await
    }

    pub async fn render_project(&self, project: &Project) -> anyhow::Result<Vec<RenderArtifact>> {
        let errors = validate_project(project);
        if !errors.is_empty() {
            let message = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("Project validation failed: {}", message);
        }

        let mut artifacts = Vec::with_capacity(project.scenes.len());
        for scene in &project.scenes {
            artifacts.push(self.render_scene(scene, &project.settings).await?);
        }
        Ok(artifacts)
    }

    pub async fn compile_scene(
        &self,
        prompt: &str,
        provider: LlmProvider,
    ) -> anyhow::Result<Scene> {
        LlmCompiler::new(provider).compile_scene(prompt).await
    }

    pub async fn patch_scene(
        &self,
        scene: &Scene,
        prompt: &str,
        provider: LlmProvider,
    ) -> anyhow::Result<Scene> {
        LlmCompiler::new(provider).patch_scene(scene, prompt).await
    }
}
