//! Snapshot history with undo/redo support.

use crate::snapshot::Snapshot;

/// An ordered history of snapshots with undo/redo cursor.
#[derive(Debug)]
pub struct History {
    snapshots: Vec<Snapshot>,
    current: usize,
}

impl History {
    /// Create an empty history.
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            current: 0,
        }
    }

    /// Push a new snapshot. Discards any redo history after current position.
    pub fn push(&mut self, snapshot: Snapshot) {
        // Truncate any future snapshots (discard redo branch).
        self.snapshots.truncate(self.current + if self.snapshots.is_empty() { 0 } else { 1 });
        self.snapshots.push(snapshot);
        self.current = self.snapshots.len() - 1;
    }

    /// Get the current snapshot.
    pub fn current(&self) -> Option<&Snapshot> {
        self.snapshots.get(self.current)
    }

    /// Undo: move back one step. Returns the snapshot we moved to.
    pub fn undo(&mut self) -> Option<&Snapshot> {
        if self.can_undo() {
            self.current -= 1;
            self.snapshots.get(self.current)
        } else {
            None
        }
    }

    /// Redo: move forward one step. Returns the snapshot we moved to.
    pub fn redo(&mut self) -> Option<&Snapshot> {
        if self.can_redo() {
            self.current += 1;
            self.snapshots.get(self.current)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.snapshots.len()
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene_ir::scene::Scene;

    fn make_snap(name: &str) -> Snapshot {
        Snapshot::capture(&Scene::new(name), Some(name.to_string()), None)
    }

    #[test]
    fn test_empty_history() {
        let history = History::new();
        assert!(history.is_empty());
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_push_and_current() {
        let mut history = History::new();
        history.push(make_snap("v1"));
        assert_eq!(history.len(), 1);
        assert_eq!(
            history.current().unwrap().message,
            Some("v1".to_string())
        );
    }

    #[test]
    fn test_undo_redo() {
        let mut history = History::new();
        history.push(make_snap("v1"));
        history.push(make_snap("v2"));
        history.push(make_snap("v3"));

        assert_eq!(history.current().unwrap().message, Some("v3".to_string()));

        history.undo();
        assert_eq!(history.current().unwrap().message, Some("v2".to_string()));

        history.undo();
        assert_eq!(history.current().unwrap().message, Some("v1".to_string()));

        assert!(!history.can_undo());

        history.redo();
        assert_eq!(history.current().unwrap().message, Some("v2".to_string()));
    }

    #[test]
    fn test_push_discards_redo() {
        let mut history = History::new();
        history.push(make_snap("v1"));
        history.push(make_snap("v2"));
        history.push(make_snap("v3"));

        history.undo(); // at v2
        history.push(make_snap("v2b")); // new branch

        assert_eq!(history.len(), 3); // v1, v2, v2b (v3 discarded)
        assert!(!history.can_redo());
    }
}
