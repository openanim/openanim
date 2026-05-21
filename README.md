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
│   └── cli/             # Command-line interface
└── python/              # Python bindings (PyO3/maturin)
```

## Quick Start

```bash
# Build the engine
cargo build --workspace

# Run tests
cargo test --workspace

# CLI usage
cargo run -p cli -- --help
cargo run -p cli -- validate my_project.json
cargo run -p cli -- schema --type-name Project
cargo run -p cli -- hash my_project.json
```

## Design Principles

1. **IR-First**: The canonical Scene IR is the source of truth. Renderers are disposable execution providers.
2. **Deterministic**: Same input always produces the same output. Content-hashing enables Bazel-style caching.
3. **Provider-Agnostic**: Support multiple renderers (Manim, Remotion, Mermaid, FFmpeg) through a unified adapter trait.
4. **Local-First**: Everything runs locally. No cloud dependency required.
5. **Incremental**: Scene graph tracks dirty nodes. Only changed subtrees are re-rendered.

## License

MIT OR Apache-2.0
