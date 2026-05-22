#pragma once

#include "openanim/llm.hpp"
#include "openanim/orchestrator.hpp"
#include "openanim/renderer.hpp"
#include "openanim/scene_ir.hpp"
#include "openanim/validation.hpp"

#include <filesystem>
#include <string_view>
#include <vector>

namespace openanim {

class OpenAnimEngine {
public:
    explicit OpenAnimEngine(
        std::filesystem::path cache_dir = ".openanim/cache",
        SandboxConfig sandbox = {});

    // ── IR validation ────────────────────────────────────────────────────────
    std::vector<ValidationError> validate_project(const Project& project) const;
    std::vector<ValidationError> validate_scene(const Scene& scene) const;

    // ── Rendering ────────────────────────────────────────────────────────────
    RenderArtifact render_scene(const Scene& scene, const RenderSettings& settings);
    RenderArtifact render_scene_with_options(const Scene& scene, const RenderSettings& settings,
                                             const RenderOptions& options);
    // Render every scene individually; returns per-scene artifacts
    std::vector<RenderArtifact> render_project(const Project& project);

    // Render all scenes then FFmpeg-concat them into one final MP4.
    // Requires at least one scene and ffmpeg on PATH.
    RenderArtifact render_project_to_video(const Project& project,
                                           const std::filesystem::path& output_path);

    // ── LLM compilation ─────────────────────────────────────────────────────
    // Compile a natural language prompt into a validated Project IR
    Project compile_project(std::string_view prompt, const LlmProvider& provider);

    // Apply a modification prompt to an existing project
    Project patch_project(const Project& current, std::string_view patch_prompt,
                          const LlmProvider& provider);

    // ── Diagnostics ──────────────────────────────────────────────────────────
    std::vector<std::string> renderer_names() const;
    std::vector<std::pair<std::string, HealthStatus>> health_check_all() const;

private:
    Orchestrator orchestrator_;
};

}  // namespace openanim
