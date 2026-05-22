#include "openanim/orchestrator.hpp"

#include <filesystem>
#include <stdexcept>

namespace openanim {
namespace {

std::string extension_for(OutputFormat format) {
    return to_string(format);
}

}  // namespace

ArtifactCache::ArtifactCache(std::filesystem::path cache_dir) : cache_dir_(std::move(cache_dir)) {
    std::filesystem::create_directories(cache_dir_);
}

std::optional<std::filesystem::path> ArtifactCache::get(std::string_view key, OutputFormat format) const {
    auto path = cache_dir_ / (std::string{key} + "." + extension_for(format));
    if (std::filesystem::exists(path)) {
        return path;
    }
    return std::nullopt;
}

std::filesystem::path ArtifactCache::put(std::string_view key, const std::filesystem::path& source, OutputFormat format) const {
    std::filesystem::create_directories(cache_dir_);
    auto target = cache_dir_ / (std::string{key} + "." + extension_for(format));
    std::filesystem::copy_file(source, target, std::filesystem::copy_options::overwrite_existing);
    return target;
}

Orchestrator::Orchestrator(std::filesystem::path cache_dir) : cache_(std::move(cache_dir)) {
    registry_.register_adapter(std::make_unique<SvgAdapter>());
    registry_.register_adapter(std::make_unique<FfmpegAdapter>());
}

RenderArtifact Orchestrator::render(const Scene& scene, const RenderSettings& settings) {
    return render_with_options(scene, settings, {});
}

RenderArtifact Orchestrator::render_with_options(const Scene& scene, const RenderSettings& settings, const RenderOptions& options) {
    auto provider_name = choose_provider(scene, options);
    auto scene_hash = hash_scene(scene);
    auto config_hash = hash_render_config(settings);
    auto key = cache_key(scene_hash, config_hash, provider_name);
    auto cache_format = OutputFormat::Svg;

    if (auto cached = cache_.get(key, cache_format)) {
        RenderArtifact artifact;
        artifact.output_path = *cached;
        artifact.format = cache_format;
        artifact.file_size_bytes = std::filesystem::file_size(*cached);
        artifact.stdout_text = "Cache hit";
        artifact.content_hash = key;
        artifact.status = ArtifactStatus::Cached;
        return artifact;
    }

    const auto* adapter = registry_.get(provider_name);
    if (!adapter) {
        throw std::runtime_error("renderer adapter is not registered: " + provider_name);
    }

    auto build_dir = std::filesystem::temp_directory_path() / "openanim-cpp-render";
    auto plan = adapter->compile(scene, settings, build_dir);
    auto artifact = adapter->execute(plan);
    auto cached = cache_.put(key, artifact.output_path, artifact.format);
    artifact.output_path = cached;
    artifact.content_hash = key;
    artifact.file_size_bytes = std::filesystem::file_size(cached);
    return artifact;
}

std::vector<std::string> Orchestrator::renderer_names() const {
    return registry_.list();
}

std::string Orchestrator::choose_provider(const Scene&, const RenderOptions& options) const {
    if (options.preferred_provider) {
        if (!registry_.get(*options.preferred_provider)) {
            throw std::runtime_error("preferred renderer is not registered: " + *options.preferred_provider);
        }
        return *options.preferred_provider;
    }
    return "svg";
}

}  // namespace openanim
