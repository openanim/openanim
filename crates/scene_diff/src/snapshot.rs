//! Immutable snapshots of scene state (like git commits).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use scene_ir::scene::Scene;

/// An immutable, timestamped capture of a scene's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique snapshot identifier.
    pub id: Uuid,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// The captured scene data (full clone).
    pub scene_data: Scene,
    /// Parent snapshot ID (for history chain).
    pub parent: Option<Uuid>,
    /// Human-readable description (like a git commit message).
    pub message: Option<String>,
}

impl Snapshot {
    /// Capture a snapshot of a scene.
    pub fn capture(scene: &Scene, message: Option<String>, parent: Option<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
            timestamp: Utc::now(),
            scene_data: scene.clone(),
            parent,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_capture() {
        let scene = Scene::new("test");
        let snap = Snapshot::capture(&scene, Some("Initial".to_string()), None);
        assert_eq!(snap.scene_data.name, "test");
        assert_eq!(snap.message, Some("Initial".to_string()));
        assert!(snap.parent.is_none());
    }

    #[test]
    fn test_snapshot_chain() {
        let scene = Scene::new("test");
        let snap1 = Snapshot::capture(&scene, Some("First".to_string()), None);
        let snap2 = Snapshot::capture(&scene, Some("Second".to_string()), Some(snap1.id));
        assert_eq!(snap2.parent, Some(snap1.id));
    }
}
