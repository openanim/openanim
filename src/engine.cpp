#include "openanim/engine.hpp"

#include <stdexcept>

namespace openanim {

OpenAnimEngine::OpenAnimEngine(std::filesystem::path cache_dir, SandboxConfig sandbox)
    : orchestrator_(std::move(cache_dir), std::move(sandbox)) {}

std::vector<ValidationError> OpenAnimEngine::validate_project(const Project& project) const {
    return openanim::validate_project(project);
}
std::vector<ValidationError> OpenAnimEngine::validate_scene(const Scene& scene) const {
    return openanim::validate_scene(scene);
}

RenderArtifact OpenAnimEngine::render_scene(const Scene& scene, const RenderSettings& settings) {
    return orchestrator_.render(scene, settings);
}
RenderArtifact OpenAnimEngine::render_scene_with_options(
    const Scene& scene, const RenderSettings& settings, const RenderOptions& options) {
    return orchestrator_.render_with_options(scene, settings, options);
}

std::vector<RenderArtifact> OpenAnimEngine::render_project(const Project& project) {
    auto errors = openanim::validate_project(project);
    if (!errors.empty()) {
        throw std::runtime_error("project validation failed: " + errors.front().message);
    }
    std::vector<RenderArtifact> artifacts;
    for (const auto& scene : project.scenes) {
        artifacts.push_back(render_scene(scene, project.settings));
    }
    return artifacts;
}

RenderArtifact OpenAnimEngine::render_project_to_video(
    const Project& project, const std::filesystem::path& output_path)
{
    // Temporarily force MP4 output for each scene
    Project mp4_project = project;
    mp4_project.settings.format = OutputFormat::Mp4;

    auto scene_artifacts = render_project(mp4_project);
    auto out_dir = output_path.parent_path().empty() ? std::filesystem::path(".") : output_path.parent_path();
    auto composed = orchestrator_.compose(scene_artifacts, out_dir);

    // Move/rename to requested output path
    if (composed.output_path != output_path) {
        std::filesystem::create_directories(output_path.parent_path());
        std::filesystem::rename(composed.output_path, output_path);
        composed.output_path = output_path;
    }
    return composed;
}

Project OpenAnimEngine::compile_project(std::string_view prompt, const LlmProvider& provider) {
    LlmCompiler compiler(provider);
    return compiler.compile_project(prompt);
}

Project OpenAnimEngine::patch_project(
    const Project& current, std::string_view patch_prompt, const LlmProvider& provider)
{
    LlmCompiler compiler(provider);
    return compiler.patch_project(current, patch_prompt);
}

std::vector<std::string> OpenAnimEngine::renderer_names() const {
    return orchestrator_.renderer_names();
}

std::vector<std::pair<std::string, HealthStatus>> OpenAnimEngine::health_check_all() const {
    return orchestrator_.health_check_all();
}

}  // namespace openanim
