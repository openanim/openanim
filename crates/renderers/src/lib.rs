//! Modular Peer Renderer Adapters for OpenAnim.
//!
//! Exposes optional adapters implementing `RendererAdapter`:
//! - `FfmpegAdapter`: Direct Scene IR to native FFmpeg command/filtergraph compiler.
//! - `ManimAdapter`: Scene IR to Python Manim compiler.
//! - `RemotionAdapter`: Scene IR to Remotion React TSX compiler.
//! - `MermaidAdapter`: Diagram node to Mermaid SVG/PNG compiler.

#[cfg(feature = "ffmpeg")]
pub mod ffmpeg;
#[cfg(feature = "manim")]
pub mod manim;
#[cfg(feature = "mermaid")]
pub mod mermaid;
#[cfg(feature = "remotion")]
pub mod remotion;

#[cfg(feature = "ffmpeg")]
pub use ffmpeg::FfmpegAdapter;
#[cfg(feature = "manim")]
pub use manim::ManimAdapter;
#[cfg(feature = "mermaid")]
pub use mermaid::MermaidAdapter;
#[cfg(feature = "remotion")]
pub use remotion::RemotionAdapter;
