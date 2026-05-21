# OpenAnim

**A deterministic multimodal animation compilation engine.**

OpenAnim is not an "AI video generator" — it's a **compilation engine** where natural language acts as source input, orchestration acts as compilation, execution providers act as runtimes, and render artifacts become the final compiled output.

## Architecture

```
Natural Language → Scene IR → Scene Graph → Renderer Adapters → Artifacts
                      ↑                           ↑
                 (like LLVM IR)            (like LLVM backends)
```

### Inspirations

| System | What it contributes |
|--------|-------------------|
| **LLVM** | Stable IR, backend abstraction, compilation passes |
| **Godot** | Scene graphs, editable node trees, dependency graphs |
| **Bazel** | Deterministic incremental builds, content-hashing, artifact caching |
| **OpenHands** | Autonomous repair loops, execution feedback |
| **ComfyUI** | Local-first composable execution graphs, plugin ecosystems |

## Crate Structure

```
openanim/
├── crates/
│   ├── scene_ir/        # Canonical renderer-agnostic IR types
│   ├── scene_graph/     # In-memory scene graph runtime
│   ├── scene_diff/      # IR versioning, diffing, patching
│   ├── hasher/          # Content-hashing for artifact caching
│   ├── renderer_core/   # Renderer adapter trait & contracts
│   ├── orchestrator/    # Engine render orchestration and cache use
│   ├── llm_compiler/    # Optional natural-language to Scene IR compiler
│   ├── engine_api/      # Embeddable API for SaaS and Rust integrations
│   └── studio/          # Optional Rust-only local developer console
└── Papers/              # Research references, not needed in runtime images
```

## Quick Start

```bash
# Build the engine
cargo build --workspace

# Build the embeddable engine API and default renderer set
cargo build -p engine_api --release

# Run tests
cargo test --workspace

# Optional local developer console
cargo run -p studio -- --dir .
```

## Deployment Boundary

The local Studio is not part of the production engine surface. SaaS code should
depend on `engine_api` directly and choose the renderer features it needs. The
old Vite/React app and general CLI have been removed. For a SaaS Docker image,
copy only the service binary using `engine_api` plus the external renderer tools
enabled for that deployment.

## Design Principles

1. **IR-First**: The canonical Scene IR is the source of truth. Renderers are disposable execution providers.
2. **Deterministic**: Same input always produces the same output. Content-hashing enables Bazel-style caching.
3. **Provider-Agnostic**: Support multiple renderers (Manim, Remotion, Mermaid, FFmpeg) through a unified adapter trait.
4. **Local-First**: Everything runs locally. No cloud dependency required.
5. **Incremental**: Scene graph tracks dirty nodes. Only changed subtrees are re-rendered.

## License

MIT OR Apache-2.0
