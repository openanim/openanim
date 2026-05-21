//! Thread-safe dynamic renderer adapter registration registry.

use std::collections::HashMap;

use crate::adapter::RendererAdapter;

/// A registry that stores and manages available renderer adapters.
#[derive(Default)]
pub struct RendererRegistry {
    adapters: HashMap<String, Box<dyn RendererAdapter>>,
}

impl RendererRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register a new renderer adapter.
    pub fn register(&mut self, adapter: Box<dyn RendererAdapter>) {
        self.adapters.insert(adapter.name().to_string(), adapter);
    }

    /// Retrieve a reference to a registered renderer adapter by name.
    pub fn get(&self, name: &str) -> Option<&dyn RendererAdapter> {
        self.adapters.get(name).map(|boxed| boxed.as_ref())
    }

    /// List the names of all registered renderer adapters.
    pub fn list(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.adapters.keys().map(|s| s.as_str()).collect();
        keys.sort();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{CompileError, ExecuteError, HealthError, HealthStatus};
    use crate::artifact::RenderArtifact;
    use crate::plan::RenderPlan;
    use async_trait::async_trait;
    use scene_ir::node::NodeType;
    use scene_ir::project::RenderSettings;
    use scene_ir::scene::Scene;

    // A mock renderer adapter for testing
    struct MockRenderer {
        name: String,
        version: String,
        supported_types: Vec<NodeType>,
    }

    #[async_trait]
    impl RendererAdapter for MockRenderer {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn supported_node_types(&self) -> &[NodeType] {
            &self.supported_types
        }

        fn compile(
            &self,
            scene: &Scene,
            _settings: &RenderSettings,
        ) -> Result<RenderPlan, CompileError> {
            Ok(RenderPlan::new(&self.name, scene.id))
        }

        async fn execute(&self, _plan: &RenderPlan) -> Result<RenderArtifact, ExecuteError> {
            Err(ExecuteError::Internal("Not implemented".to_string()))
        }

        async fn health_check(&self) -> Result<HealthStatus, HealthError> {
            Ok(HealthStatus {
                available: true,
                version: Some(self.version.clone()),
                message: None,
            })
        }
    }

    #[test]
    fn test_registry_registration_and_lookup() {
        let mut registry = RendererRegistry::new();
        assert!(registry.list().is_empty());
        assert!(registry.get("mock").is_none());

        let mock = MockRenderer {
            name: "mock".to_string(),
            version: "1.0.0".to_string(),
            supported_types: vec![NodeType::Shape],
        };

        registry.register(Box::new(mock));

        assert_eq!(registry.list(), vec!["mock"]);
        let retrieved = registry.get("mock").unwrap();
        assert_eq!(retrieved.name(), "mock");
        assert_eq!(retrieved.version(), "1.0.0");
        assert_eq!(retrieved.supported_node_types(), &[NodeType::Shape]);
    }

    #[test]
    fn test_registry_multiple_registration() {
        let mut registry = RendererRegistry::new();

        registry.register(Box::new(MockRenderer {
            name: "manim".to_string(),
            version: "0.18.1".to_string(),
            supported_types: vec![],
        }));

        registry.register(Box::new(MockRenderer {
            name: "ffmpeg".to_string(),
            version: "7.0".to_string(),
            supported_types: vec![],
        }));

        let list = registry.list();
        assert_eq!(list, vec!["ffmpeg", "manim"]);

        let ffmpeg = registry.get("ffmpeg").unwrap();
        assert_eq!(ffmpeg.name(), "ffmpeg");
        assert_eq!(ffmpeg.version(), "7.0");

        let manim = registry.get("manim").unwrap();
        assert_eq!(manim.name(), "manim");
        assert_eq!(manim.version(), "0.18.1");
    }
}
