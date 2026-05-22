#pragma once

#include "openanim/orchestrator.hpp"
#include "openanim/validation.hpp"

namespace openanim {

class OpenAnimEngine {
public:
    explicit OpenAnimEngine(std::filesystem::path cache_dir);

    std::vector<std::string> renderer_names() const;
    std::vector<ValidationError> validate_project(const Project& project) const;
    std::vector<ValidationError> validate_scene(const Scene& scene) const;
    RenderArtifact render_scene(const Scene& scene, const RenderSettings& settings);
    RenderArtifact render_scene_with_options(const Scene& scene, const RenderSettings& settings, const RenderOptions& options);
    std::vector<RenderArtifact> render_project(const Project& project);

private:
    Orchestrator orchestrator_;
};

}  // namespace openanim
