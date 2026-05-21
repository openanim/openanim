//! Modular Peer Renderer Adapters for OpenAnim.
//!
//! Exposes four adapters implementing `RendererAdapter`:
//! - `FfmpegAdapter`: Direct Scene IR to native FFmpeg command/filtergraph compiler.
//! - `ManimAdapter`: Scene IR to Python Manim compiler.
//! - `RemotionAdapter`: Scene IR to Remotion React TSX compiler.
//! - `MermaidAdapter`: Diagram node to Mermaid SVG/PNG compiler.

pub mod ffmpeg;
pub mod manim;
pub mod remotion;
pub mod mermaid;

pub use ffmpeg::FfmpegAdapter;
pub use manim::ManimAdapter;
pub use remotion::RemotionAdapter;
pub use mermaid::MermaidAdapter;
