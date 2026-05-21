//! # OpenAnim Scene IR
//!
//! The canonical, renderer-agnostic intermediate representation for OpenAnim.
//!
//! This crate defines every type in the scene IR. The IR is the permanent,
//! editable source of truth — renderers are disposable execution providers
//! that compile from this IR. Think of this as the "LLVM IR" of animation.
//!
//! ## Architecture
//!
//! ```text
//! Project
//! ├── ProjectMetadata (name, description, version)
//! ├── RenderSettings (resolution, fps, format)
//! ├── AssetManifest (external asset references)
//! └── Vec<Scene>
//!     ├── SceneMetadata
//!     ├── root_node: NodeId → Node tree
//!     │   ├── Node { id, type, components, children }
//!     │   │   ├── Transform (position, rotation, scale)
//!     │   │   ├── Style (fill, stroke, opacity)
//!     │   │   ├── Shape / Text / MathExpr / Image
//!     │   │   └── ...
//!     │   └── children: Vec<NodeId>
//!     └── Timeline
//!         ├── Vec<KeyframeTrack> (continuous animations)
//!         └── Vec<AnimationEvent> (discrete transitions)
//! ```

pub mod components;
pub mod node;
pub mod project;
pub mod scene;
pub mod schema;
pub mod timeline;
pub mod types;
pub mod validation;

// Re-export primary types for convenience.
pub use components::*;
pub use node::{ComponentSet, Node, NodeType};
pub use project::{AssetManifest, Project, ProjectMetadata, RenderSettings};
pub use scene::{Scene, SceneMetadata};
pub use timeline::{
    AnimatableProperty, AnimationEvent, EasingFunction, EventParams, EventType, Keyframe,
    KeyframeTrack, Timeline,
};
pub use types::*;
