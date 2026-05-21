//! # OpenAnim Scene Diff
//!
//! IR versioning, structural diffing, patching, and snapshot history.
//! Enables semantic edits (patches) instead of full regeneration.

pub mod diff;
pub mod history;
pub mod patch;
pub mod snapshot;

pub use diff::{DiffOp, SceneDiff};
pub use history::History;
pub use patch::{PatchError, apply_diff};
pub use snapshot::Snapshot;
