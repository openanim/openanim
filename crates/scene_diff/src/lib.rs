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
pub use patch::{apply_diff, PatchError};
pub use snapshot::Snapshot;
