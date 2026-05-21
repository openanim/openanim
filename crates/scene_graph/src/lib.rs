//! # OpenAnim Scene Graph Runtime
//!
//! Persistent in-memory representation of the scene node tree.
//! Provides arena-allocated storage, hierarchy management,
//! dirty tracking, queries, and a fluent builder API.

pub mod builder;
pub mod dirty;
pub mod graph;
pub mod query;

pub use builder::SceneGraphBuilder;
pub use dirty::{DirtyFlags, DirtyTracker};
pub use graph::SceneGraph;
