//! # OpenAnim Renderer Core
//!
//! Defines the `RendererAdapter` trait — the contract that all execution
//! providers (Manim, Remotion, Mermaid, FFmpeg, etc.) must implement.
//!
//! Also defines `RenderPlan` (compiled instructions) and `RenderArtifact`
//! (output from execution).

pub mod adapter;
pub mod artifact;
pub mod plan;
pub mod registry;

pub use adapter::{CompileError, ExecuteError, HealthError, HealthStatus, RendererAdapter};
pub use artifact::{ArtifactStatus, RenderArtifact};
pub use plan::{RenderCommand, RenderPlan};
pub use registry::RendererRegistry;
