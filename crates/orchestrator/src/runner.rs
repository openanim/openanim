//! Scene compile and execution Orchestrator.

use std::path::PathBuf;

use scene_ir::scene::Scene;
use scene_ir::project::RenderSettings;
use scene_ir::node::NodeType;
use scene_ir::components::ImageContent;
use scene_ir::types::{AssetRef, ImageFit};

use renderer_core::registry::RendererRegistry;
use renderer_core::artifact::{RenderArtifact, ArtifactStatus};

use crate::cache::ArtifactCache;

pub struct Orchestrator {
    pub registry: RendererRegistry,
    pub cache: ArtifactCache,
}

impl Orchestrator {
    pub fn new(cache_dir: PathBuf) -> Self {
        let mut registry = RendererRegistry::new();
        registry.register(Box::new(renderers::FfmpegAdapter::new()));
        registry.register(Box::new(renderers::MermaidAdapter::new()));
        registry.register(Box::new(renderers::ManimAdapter::new()));
        registry.register(Box::new(renderers::RemotionAdapter::new()));

        let cache = ArtifactCache::new(cache_dir);

        Self { registry, cache }
    }

    /// Renders a Scene graph, utilizing Bazel-style caching and graph partitioning.
    pub async fn render(&self, scene: &Scene, settings: &RenderSettings) -> Result<RenderArtifact, anyhow::Error> {
        let scene_hash = hasher::hash_scene(scene);
        let config_hash = hasher::hash_render_config(settings);

        // Determine final composition provider:
        // Remotion and Manim are peer renderers, but if specialized Mermaid nodes exist,
        // we sub-render them to SVGs and layer them via FFmpeg.
        let provider_name = if scene.nodes.iter().any(|n| n.node_type == NodeType::Diagram) {
            "ffmpeg" // Fallback to FFmpeg as main laying linking engine
        } else {
            "ffmpeg" // Default compositing engine
        };

        let cache_key = hasher::cache_key(&scene_hash, &config_hash, provider_name);

        // 1. Cache hit check
        if let Some(cached_file) = self.cache.get(&cache_key) {
            return Ok(RenderArtifact {
                id: uuid::Uuid::now_v7(),
                plan_id: uuid::Uuid::now_v7(),
                output_path: cached_file,
                format: scene_ir::types::OutputFormat::Mp4,
                file_size_bytes: Some(0),
                render_duration: std::time::Duration::from_secs(0),
                stdout: "Cache hit: retrieved from Merkle cache substrate".to_string(),
                stderr: String::new(),
                exit_code: 0,
                content_hash: Some(cache_key),
                status: ArtifactStatus::Success,
            });
        }

        // 2. Graph Partitioning
        // Partition specialized nodes (e.g. NodeType::Diagram) to Mermaid renderer
        let mut composition_scene = scene.clone();

        for node in &mut composition_scene.nodes {
            if node.node_type == NodeType::Diagram {
                // If it is a Mermaid diagram, sub-render it first!
                if let Some(diagram) = &node.components.diagram {
                    if diagram.language == scene_ir::components::DiagramLanguage::Mermaid {
                        let mermaid_adapter = self.registry.get("mermaid")
                            .ok_or_else(|| anyhow::anyhow!("Mermaid adapter not registered"))?;

                        // Create mini-scene containing only this diagram node for the mermaid compiler
                        let mut sub_scene = Scene::new("sub_diagram");
                        sub_scene.nodes.push(node.clone());

                        let plan = mermaid_adapter.compile(&sub_scene, settings)
                            .map_err(|e| anyhow::anyhow!("Failed to compile Mermaid sub-scene: {:?}", e))?;

                        let sub_artifact = mermaid_adapter.execute(&plan).await
                            .map_err(|e| anyhow::anyhow!("Failed to execute Mermaid sub-render: {:?}", e))?;

                        // Replace node in composite scene with NodeType::Image pointing to output SVG
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
                }
            }
        }

        // 3. Main Composition Render
        let main_adapter = self.registry.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Main adapter '{}' not found", provider_name))?;

        let main_plan = main_adapter.compile(&composition_scene, settings)
            .map_err(|e| anyhow::anyhow!("Failed to compile composition scene: {:?}", e))?;

        let mut artifact = main_adapter.execute(&main_plan).await
            .map_err(|e| anyhow::anyhow!("Failed to execute composition render: {:?}", e))?;

        // 4. Cache populate
        let format_str = match artifact.format {
            scene_ir::types::OutputFormat::Mp4 => "mp4",
            scene_ir::types::OutputFormat::Svg => "svg",
            scene_ir::types::OutputFormat::Png => "png",
            _ => "mp4",
        };

        if let Ok(cached_file) = self.cache.put(&cache_key, &artifact.output_path, format_str) {
            artifact.output_path = cached_file;
            artifact.content_hash = Some(cache_key);
        }

        Ok(artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use scene_ir::node::Node;
    use scene_ir::components::{Shape, Diagram, DiagramLanguage};

    #[tokio::test]
    async fn test_orchestrator_cache_hit_and_graph_partitioning() {
        let cache_dir = tempdir().unwrap();
        let orchestrator = Orchestrator::new(cache_dir.path().to_path_buf());

        // Construct scene with specialized Mermaid Diagram node
        let mut scene = Scene::new("Graph Partitioning Scene");
        let mut diag_node = Node::new(NodeType::Diagram);
        diag_node.components.diagram = Some(Diagram {
            source: "graph LR; Start --> Stop;".to_string(),
            language: DiagramLanguage::Mermaid,
        });
        scene.nodes.push(diag_node);

        // Also add a shape node
        let mut shape_node = Node::new(NodeType::Shape);
        shape_node.components.shape = Some(Shape {
            kind: scene_ir::components::ShapeKind::Circle { radius: 10.0 },
        });
        scene.nodes.push(shape_node);

        let settings = RenderSettings::default();

        // Perform rendering (this will cache miss, run sub-render, compile main scene)
        let artifact = orchestrator.render(&scene, &settings).await.unwrap();
        assert_eq!(artifact.status, ArtifactStatus::Success);
        assert!(artifact.content_hash.is_some());

        // Perform second rendering to assert CACHE HIT
        let cached_artifact = orchestrator.render(&scene, &settings).await.unwrap();
        assert_eq!(cached_artifact.status, ArtifactStatus::Success);
        assert_eq!(cached_artifact.content_hash, artifact.content_hash);
        assert!(cached_artifact.stdout.contains("Cache hit"));
    }
}

