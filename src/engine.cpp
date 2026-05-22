#include "openanim/engine.hpp"

#include <stdexcept>

namespace openanim {

OpenAnimEngine::OpenAnimEngine(std::filesystem::path cache_dir) : orchestrator_(std::move(cache_dir)) {}

std::vector<std::string> OpenAnimEngine::renderer_names() const {
    return orchestrator_.renderer_names();
}

std::vector<ValidationError> OpenAnimEngine::validate_project(const Project& project) const {
    return openanim::validate_project(project);
}

std::vector<ValidationError> OpenAnimEngine::validate_scene(const Scene& scene) const {
    return openanim::validate_scene(scene);
}

RenderArtifact OpenAnimEngine::render_scene(const Scene& scene, const RenderSettings& settings) {
    return orchestrator_.render(scene, settings);
}

RenderArtifact OpenAnimEngine::render_scene_with_options(const Scene& scene, const RenderSettings& settings, const RenderOptions& options) {
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

}  // namespace openanim
