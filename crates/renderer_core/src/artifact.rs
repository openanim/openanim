//! Render artifact: output from executing a render plan.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use scene_ir::types::OutputFormat;

/// The status of a rendered artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ArtifactStatus {
    Success,
    Failed { error: String },
    Timeout,
}

/// The output produced by executing a render plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderArtifact {
    /// Unique artifact identifier.
    pub id: Uuid,
    /// The plan that produced this artifact.
    pub plan_id: Uuid,
    /// Path to the output file.
    pub output_path: PathBuf,
    /// Output format.
    pub format: OutputFormat,
    /// File size in bytes (if available).
    pub file_size_bytes: Option<u64>,
    /// How long rendering took.
    pub render_duration: Duration,
    /// Combined stdout from all process executions.
    pub stdout: String,
    /// Combined stderr from all process executions.
    pub stderr: String,
    /// Final process exit code.
    pub exit_code: i32,
    /// Content hash of the output file (for caching).
    pub content_hash: Option<String>,
    /// Overall status.
    pub status: ArtifactStatus,
}

impl RenderArtifact {
    /// Check if the render was successful.
    pub fn is_success(&self) -> bool {
        matches!(self.status, ArtifactStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_status_serde() {
        let success = ArtifactStatus::Success;
        let json = serde_json::to_string(&success).unwrap();
        let deserialized: ArtifactStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ArtifactStatus::Success);

        let failed = ArtifactStatus::Failed {
            error: "oops".to_string(),
        };
        let json = serde_json::to_string(&failed).unwrap();
        let deserialized: ArtifactStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized,
            ArtifactStatus::Failed {
                error: "oops".to_string()
            }
        );
    }

    #[test]
    fn test_render_artifact_serde() {
        let artifact = RenderArtifact {
            id: Uuid::now_v7(),
            plan_id: Uuid::now_v7(),
            output_path: PathBuf::from("/output/scene.mp4"),
            format: OutputFormat::Mp4,
            file_size_bytes: Some(1024 * 1024),
            render_duration: Duration::from_secs(15),
            stdout: "Rendering complete".to_string(),
            stderr: String::new(),
            exit_code: 0,
            content_hash: Some("abc123".to_string()),
            status: ArtifactStatus::Success,
        };

        let json = serde_json::to_string_pretty(&artifact).unwrap();
        let deserialized: RenderArtifact = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_success());
    }
}
