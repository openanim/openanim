//! Scene compile and execution orchestration.

use std::path::PathBuf;

use renderer_core::artifact::{ArtifactStatus, RenderArtifact};
use renderer_core::registry::RendererRegistry;
use scene_ir::components::{DiagramLanguage, ImageContent};
use scene_ir::node::NodeType;
use scene_ir::project::RenderSettings;
use scene_ir::scene::Scene;
use scene_ir::types::{AssetRef, ImageFit};

use crate::cache::ArtifactCache;

#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    pub preferred_provider: Option<String>,
}

pub struct Orchestrator {
    pub registry: RendererRegistry,
    pub cache: ArtifactCache,
}

impl Orchestrator {
    pub fn new(cache_dir: PathBuf) -> Self {
        let mut registry = RendererRegistry::new();

        #[cfg(feature = "ffmpeg")]
        registry.register(Box::new(renderers::FfmpegAdapter::new()));
        #[cfg(feature = "mermaid")]
        registry.register(Box::new(renderers::MermaidAdapter::new()));
        #[cfg(feature = "manim")]
        registry.register(Box::new(renderers::ManimAdapter::new()));
        #[cfg(feature = "remotion")]
        registry.register(Box::new(renderers::RemotionAdapter::new()));

        Self {
            registry,
            cache: ArtifactCache::new(cache_dir),
        }
    }

    pub async fn render(
        &self,
        scene: &Scene,
        settings: &RenderSettings,
    ) -> Result<RenderArtifact, anyhow::Error> {
        self.render_with_options(scene, settings, &RenderOptions::default())
            .await
    }

    /// Renders a scene using the best registered provider for the scene shape.
    ///
    /// FFmpeg is preferred as the compositor/final artifact provider for ordinary
    /// 2D scenes. Specialized providers are used only when the scene benefits
    /// from them and the adapter was compiled into this build.
    pub async fn render_with_options(
        &self,
        scene: &Scene,
        settings: &RenderSettings,
        options: &RenderOptions,
    ) -> Result<RenderArtifact, anyhow::Error> {
        let provider_name = self.choose_provider(scene, options)?;
        let scene_hash = hasher::hash_scene(scene);
        let config_hash = hasher::hash_render_config(settings);
        let cache_key = hasher::cache_key(&scene_hash, &config_hash, &provider_name);

        if let Some(cached_file) = self.cache.get(&cache_key) {
            return Ok(RenderArtifact {
                id: uuid::Uuid::now_v7(),
                plan_id: uuid::Uuid::now_v7(),
                output_path: cached_file,
                format: scene_ir::types::OutputFormat::Mp4,
                file_size_bytes: Some(0),
                render_duration: std::time::Duration::from_secs(0),
                stdout: "Cache hit: retrieved from content-addressed render cache".to_string(),
                stderr: String::new(),
                exit_code: 0,
                content_hash: Some(cache_key),
                status: ArtifactStatus::Success,
            });
        }

        let mut composition_scene = scene.clone();
        self.materialize_specialized_nodes(&mut composition_scene, settings)
            .await?;

        let main_adapter = self.registry.get(&provider_name).ok_or_else(|| {
            anyhow::anyhow!("Renderer adapter '{}' is not registered", provider_name)
        })?;

        let main_plan = main_adapter
            .compile(&composition_scene, settings)
            .map_err(|e| {
                anyhow::anyhow!("Failed to compile scene with {}: {:?}", provider_name, e)
            })?;

        let mut artifact = main_adapter
            .execute(&main_plan)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute {} render: {:?}", provider_name, e))?;

        let format_str = match artifact.format {
            scene_ir::types::OutputFormat::Mp4 => "mp4",
            scene_ir::types::OutputFormat::Svg => "svg",
            scene_ir::types::OutputFormat::Png => "png",
            _ => "mp4",
        };

        if let Ok(cached_file) = self
            .cache
            .put(&cache_key, &artifact.output_path, format_str)
        {
            artifact.output_path = cached_file;
            artifact.content_hash = Some(cache_key);
        }

        Ok(artifact)
    }

    fn choose_provider(
        &self,
        scene: &Scene,
        options: &RenderOptions,
    ) -> Result<String, anyhow::Error> {
        if let Some(provider) = &options.preferred_provider {
            if self.registry.get(provider).is_some() {
                return Ok(provider.clone());
            }
            anyhow::bail!("Preferred renderer '{}' is not registered", provider);
        }

        let has_math_or_code = scene
            .nodes
            .iter()
            .any(|node| matches!(node.node_type, NodeType::Math | NodeType::Code));
        if has_math_or_code && self.registry.get("manim").is_some() {
            return Ok("manim".to_string());
        }

        let has_rich_web_nodes = scene.nodes.iter().any(|node| {
            node.node_type == NodeType::Custom && self.registry.get("remotion").is_some()
        });
        if has_rich_web_nodes {
            return Ok("remotion".to_string());
        }

        if self.registry.get("ffmpeg").is_some() {
            return Ok("ffmpeg".to_string());
        }

        self.registry
            .list()
            .first()
            .map(|name| (*name).to_string())
            .ok_or_else(|| anyhow::anyhow!("No renderer adapters are registered"))
    }

    async fn materialize_specialized_nodes(
        &self,
        scene: &mut Scene,
        settings: &RenderSettings,
    ) -> Result<(), anyhow::Error> {
        for node in &mut scene.nodes {
            if node.node_type != NodeType::Diagram {
                continue;
            }

            let Some(diagram) = &node.components.diagram else {
                continue;
            };

            if diagram.language != DiagramLanguage::Mermaid {
                continue;
            }

            let Some(mermaid_adapter) = self.registry.get("mermaid") else {
                continue;
            };

            let mut sub_scene = Scene::new("sub_diagram");
            sub_scene.nodes.push(node.clone());

            let plan = mermaid_adapter
                .compile(&sub_scene, settings)
                .map_err(|e| anyhow::anyhow!("Failed to compile Mermaid diagram: {:?}", e))?;

            let sub_artifact = mermaid_adapter
                .execute(&plan)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to render Mermaid diagram: {:?}", e))?;

            node.node_type = NodeType::Image;
            node.components.image = Some(ImageContent {
                asset_ref: AssetRef {
                    asset_id: uuid::Uuid::now_v7().to_string(),
                    path: sub_artifact.output_path.to_string_lossy().into_owned(),
                    mime_type: Some("image/svg+xml".to_string()),
                    content_hash: None,
                },
                fit: ImageFit::Contain,
                width: None,
                height: None,
            });
            node.components.diagram = None;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::components::{Diagram, DiagramLanguage, Shape};
    use scene_ir::node::Node;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_orchestrator_cache_hit() {
        let cache_dir = tempdir().unwrap();
        let orchestrator = Orchestrator::new(cache_dir.path().to_path_buf());

        let mut scene = Scene::new("Graph Partitioning Scene");
        let mut diag_node = Node::new(NodeType::Diagram);
        diag_node.components.diagram = Some(Diagram {
            source: "graph LR; Start --> Stop;".to_string(),
            language: DiagramLanguage::Mermaid,
        });
        scene.nodes.push(diag_node);

        let mut shape_node = Node::new(NodeType::Shape);
        shape_node.components.shape = Some(Shape {
            kind: scene_ir::components::ShapeKind::Circle { radius: 10.0 },
        });
        scene.nodes.push(shape_node);

        let settings = RenderSettings::default();
        let artifact = orchestrator.render(&scene, &settings).await.unwrap();
        assert_eq!(artifact.status, ArtifactStatus::Success);
        assert!(artifact.content_hash.is_some());

        let cached_artifact = orchestrator.render(&scene, &settings).await.unwrap();
        assert_eq!(cached_artifact.status, ArtifactStatus::Success);
        assert_eq!(cached_artifact.content_hash, artifact.content_hash);
        assert!(cached_artifact.stdout.contains("Cache hit"));
    }
}
