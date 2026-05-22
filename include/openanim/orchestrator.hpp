#pragma once

#include "openanim/hasher.hpp"
#include "openanim/renderer.hpp"

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
    explicit Orchestrator(std::filesystem::path cache_dir);

    RenderArtifact render(const Scene& scene, const RenderSettings& settings);
    RenderArtifact render_with_options(const Scene& scene, const RenderSettings& settings, const RenderOptions& options);
    std::vector<std::string> renderer_names() const;

private:
    std::string choose_provider(const Scene& scene, const RenderOptions& options) const;

    RendererRegistry registry_;
    ArtifactCache cache_;
};

}  // namespace openanim
