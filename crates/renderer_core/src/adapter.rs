//! The core renderer adapter trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use scene_ir::node::NodeType;
use scene_ir::project::RenderSettings;
use scene_ir::scene::Scene;

use crate::artifact::RenderArtifact;
use crate::plan::RenderPlan;

/// Errors during scene compilation (IR → RenderPlan).
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Unsupported node type: {0:?}")]
    UnsupportedNodeType(NodeType),
    #[error("Invalid scene: {0}")]
    InvalidScene(String),
    #[error("Missing required component on node: {0}")]
    MissingComponent(String),
    #[error("Internal compiler error: {0}")]
    Internal(String),
}

/// Errors during plan execution (RenderPlan → RenderArtifact).
#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("Process failed with exit code {exit_code}: {stderr}")]
    ProcessFailed { exit_code: i32, stderr: String },
    #[error("Execution timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },
    #[error("File I/O error: {0}")]
    IoError(String),
    #[error("Internal execution error: {0}")]
    Internal(String),
}

/// Errors during health check.
#[derive(Debug, Error)]
pub enum HealthError {
    #[error("Provider not installed: {0}")]
    NotInstalled(String),
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    #[error("Health check failed: {0}")]
    CheckFailed(String),
}

/// Result of a health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub available: bool,
    pub version: Option<String>,
    pub message: Option<String>,
}

/// The core trait that all renderer providers implement.
///
/// This is the "backend" in the LLVM analogy: each renderer
/// (Manim, Remotion, Mermaid, FFmpeg, etc.) implements this trait
/// to compile Scene IR into executable plans and produce artifacts.
#[async_trait]
pub trait RendererAdapter: Send + Sync {
    /// Human-readable name (e.g., "manim", "remotion").
    fn name(&self) -> &str;

    /// Provider version string.
    fn version(&self) -> &str;

    /// Which node types this renderer can handle.
    fn supported_node_types(&self) -> &[NodeType];

    /// Compile a scene + settings into a render plan.
    fn compile(&self, scene: &Scene, settings: &RenderSettings)
    -> Result<RenderPlan, CompileError>;

    /// Execute a render plan, producing an artifact.
    async fn execute(&self, plan: &RenderPlan) -> Result<RenderArtifact, ExecuteError>;

    /// Check if this provider is available and ready.
    async fn health_check(&self) -> Result<HealthStatus, HealthError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serde() {
        let status = HealthStatus {
            available: true,
            version: Some("0.18.1".to_string()),
            message: Some("Ready".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.available, true);
        assert_eq!(deserialized.version, Some("0.18.1".to_string()));
    }
}
