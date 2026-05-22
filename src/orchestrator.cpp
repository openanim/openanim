#include "openanim/orchestrator.hpp"

#include <filesystem>
#include <fstream>
#include <stdexcept>

namespace openanim {
namespace {

std::string extension_for(OutputFormat format) { return to_string(format); }

// Scan scene nodes and decide which adapter is most appropriate
std::string infer_provider(const Scene& scene, const RendererRegistry& registry) {
    bool has_mermaid = false, has_plantuml = false, has_math = false;
    for (const auto& node : scene.nodes) {
        if (node.components.diagram) {
            if (node.components.diagram->language == DiagramLanguage::Mermaid) has_mermaid = true;
            if (node.components.diagram->language == DiagramLanguage::PlantUml) has_plantuml = true;
        }
        if (node.components.math) has_math = true;
    }

    // Priority: PlantUML → Mermaid → Manim → SVG (always available fallback)
    if (has_plantuml && registry.get("plantuml")) return "plantuml";
    if (has_mermaid  && registry.get("mermaid"))  return "mermaid";
    if (has_math     && registry.get("manim"))    return "manim";
    return "svg";
}

}  // namespace

// ─── ArtifactCache ────────────────────────────────────────────────────────────

ArtifactCache::ArtifactCache(std::filesystem::path cache_dir)
    : cache_dir_(std::move(cache_dir)) {
    std::filesystem::create_directories(cache_dir_);
}

std::optional<std::filesystem::path> ArtifactCache::get(
    std::string_view key, OutputFormat format) const
{
    auto path = cache_dir_ / (std::string{key} + "." + extension_for(format));
    if (std::filesystem::exists(path)) return path;
    return std::nullopt;
}

std::filesystem::path ArtifactCache::put(
    std::string_view key, const std::filesystem::path& source, OutputFormat format) const
{
    std::filesystem::create_directories(cache_dir_);
    auto target = cache_dir_ / (std::string{key} + "." + extension_for(format));
    std::filesystem::copy_file(source, target, std::filesystem::copy_options::overwrite_existing);
    return target;
}

// ─── Orchestrator ─────────────────────────────────────────────────────────────

Orchestrator::Orchestrator(std::filesystem::path cache_dir, SandboxConfig sandbox)
    : cache_(std::move(cache_dir)), sandbox_(std::move(sandbox))
{
    // Register all adapters, propagating the sandbox config
    auto svg = std::make_unique<SvgAdapter>();
    svg->sandbox = sandbox_;
    registry_.register_adapter(std::move(svg));

    auto ffmpeg = std::make_unique<FfmpegAdapter>();
    ffmpeg->sandbox = sandbox_;
    registry_.register_adapter(std::move(ffmpeg));

    auto mermaid = std::make_unique<MermaidAdapter>();
    mermaid->sandbox = sandbox_;
    registry_.register_adapter(std::move(mermaid));

    auto plantuml = std::make_unique<PlantUmlAdapter>();
    plantuml->sandbox = sandbox_;
    registry_.register_adapter(std::move(plantuml));

    auto manim = std::make_unique<ManimAdapter>();
    manim->sandbox = sandbox_;
    registry_.register_adapter(std::move(manim));

    auto remotion = std::make_unique<RemotionAdapter>();
    remotion->sandbox = sandbox_;
    registry_.register_adapter(std::move(remotion));
}

RenderArtifact Orchestrator::render(const Scene& scene, const RenderSettings& settings) {
    return render_with_options(scene, settings, {});
}

RenderArtifact Orchestrator::render_with_options(
    const Scene& scene, const RenderSettings& settings, const RenderOptions& options)
{
    auto provider_name = choose_provider(scene, options);
    auto scene_hash    = hash_scene(scene);
    auto config_hash   = hash_render_config(settings);
    auto key           = cache_key(scene_hash, config_hash, provider_name);
    auto cache_format  = settings.format;

    if (auto cached = cache_.get(key, cache_format)) {
        RenderArtifact art;
        art.output_path     = *cached;
        art.format          = cache_format;
        art.file_size_bytes = std::filesystem::file_size(*cached);
        art.stdout_text     = "Cache hit";
        art.content_hash    = key;
        art.status          = ArtifactStatus::Cached;
        return art;
    }

    const auto* adapter = registry_.get(provider_name);
    if (!adapter) throw std::runtime_error("renderer adapter not registered: " + provider_name);

    auto build_dir = std::filesystem::temp_directory_path() / "openanim-render";
    auto plan      = adapter->compile(scene, settings, build_dir);
    auto artifact  = adapter->execute(plan);
    auto cached    = cache_.put(key, artifact.output_path, artifact.format);
    artifact.output_path     = cached;
    artifact.content_hash    = key;
    artifact.file_size_bytes = std::filesystem::file_size(cached);
    return artifact;
}

std::vector<std::string> Orchestrator::renderer_names() const { return registry_.list(); }

std::vector<std::pair<std::string, HealthStatus>> Orchestrator::health_check_all() const {
    std::vector<std::pair<std::string, HealthStatus>> results;
    for (const auto& name : registry_.list()) {
        const auto* adapter = registry_.get(name);
        if (adapter) results.emplace_back(name, adapter->health_check());
    }
    return results;
}

std::string Orchestrator::choose_provider(
    const Scene& scene, const RenderOptions& options) const
{
    if (options.preferred_provider) {
        if (!registry_.get(*options.preferred_provider)) {
            throw std::runtime_error("preferred renderer not registered: " + *options.preferred_provider);
        }
        return *options.preferred_provider;
    }
    return infer_provider(scene, registry_);
}

RenderArtifact Orchestrator::compose(
    const std::vector<RenderArtifact>& artifacts,
    const std::filesystem::path& output_dir)
{
    if (artifacts.empty()) throw std::runtime_error("compose: no artifacts to compose");
    if (artifacts.size() == 1) return artifacts.front();

    // Build concat list file
    auto work = std::filesystem::temp_directory_path() / "openanim-compose";
    std::filesystem::create_directories(work);
    auto list_path = work / "concat.txt";
    { std::ofstream f(list_path);
      for (const auto& art : artifacts)
          f << "file '" << art.output_path.string() << "'\n"; }

    std::filesystem::create_directories(output_dir);
    auto out = output_dir / "final.mp4";

    auto result = run_command("ffmpeg", {
        "-y", "-f", "concat", "-safe", "0",
        "-i", list_path.string(),
        "-c", "copy",
        out.string()
    }, work, {});

    std::filesystem::remove_all(work);
    if (result.exit_code != 0)
        throw std::runtime_error("FFmpeg compose failed: " + result.stderr_text);

    RenderArtifact art;
    art.output_path     = out;
    art.format          = OutputFormat::Mp4;
    art.file_size_bytes = std::filesystem::file_size(out);
    art.stdout_text     = result.stdout_text;
    return art;
}

}  // namespace openanim
