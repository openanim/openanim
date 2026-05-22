#pragma once

#include "openanim/hasher.hpp"
#include "openanim/renderer.hpp"
#include "openanim/sandbox.hpp"

namespace openanim {

struct RenderOptions {
    std::optional<std::string> preferred_provider;
};

class ArtifactCache {
public:
    explicit ArtifactCache(std::filesystem::path cache_dir);
    std::optional<std::filesystem::path> get(std::string_view key, OutputFormat format) const;
    std::filesystem::path put(std::string_view key, const std::filesystem::path& source, OutputFormat format) const;
private:
    std::filesystem::path cache_dir_;
};

class Orchestrator {
public:
    explicit Orchestrator(std::filesystem::path cache_dir,
                          SandboxConfig sandbox = {});

    RenderArtifact render(const Scene& scene, const RenderSettings& settings);
    RenderArtifact render_with_options(const Scene& scene, const RenderSettings& settings,
                                       const RenderOptions& options);
    // Combine multiple scene artifacts into one final MP4 via FFmpeg concat
    RenderArtifact compose(const std::vector<RenderArtifact>& artifacts,
                           const std::filesystem::path& output_dir);
    std::vector<std::string> renderer_names() const;

    // Health check all registered adapters
    std::vector<std::pair<std::string, HealthStatus>> health_check_all() const;

private:
    // Examine scene node types and pick the best available adapter
    std::string choose_provider(const Scene& scene, const RenderOptions& options) const;

    RendererRegistry registry_;
    ArtifactCache cache_;
    SandboxConfig sandbox_;
};

}  // namespace openanim
