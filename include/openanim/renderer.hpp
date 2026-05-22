#pragma once

#include "openanim/sandbox.hpp"
#include "openanim/scene_ir.hpp"

#include <chrono>
#include <filesystem>
#include <memory>
#include <string>
#include <unordered_map>

namespace openanim {

enum class ArtifactStatus { Success, Failed, Cached };

struct RenderArtifact {
    Id id = Id::make();
    Id plan_id = Id::make();
    std::filesystem::path output_path;
    OutputFormat format = OutputFormat::Svg;
    std::uintmax_t file_size_bytes = 0;
    std::chrono::milliseconds render_duration{0};
    std::string stdout_text;
    std::string stderr_text;
    int exit_code = 0;
    std::optional<ContentHash> content_hash;
    ArtifactStatus status = ArtifactStatus::Success;
};

struct RenderCommand {
    std::string program;
    std::vector<std::string> args;
    std::filesystem::path working_directory;
};

struct RenderPlan {
    Id id = Id::make();
    std::string provider;
    OutputFormat output_format = OutputFormat::Svg;
    std::filesystem::path output_path;
    std::vector<RenderCommand> commands;
    std::optional<std::string> inline_svg;
    // Extra metadata used by individual adapters
    std::optional<std::string> source_text;  // Mermaid / PlantUML source
};

struct HealthStatus {
    bool available = true;
    std::optional<std::string> version;
    std::optional<std::string> message;
};

class RendererAdapter {
public:
    virtual ~RendererAdapter() = default;
    virtual std::string name() const = 0;
    virtual std::string version() const = 0;
    virtual std::vector<NodeType> supported_node_types() const = 0;
    virtual RenderPlan compile(const Scene& scene, const RenderSettings& settings,
                               const std::filesystem::path& output_dir) const = 0;
    virtual RenderArtifact execute(const RenderPlan& plan) const = 0;
    virtual HealthStatus health_check() const = 0;

    // Sandbox configuration (defaults to DirectProcess; set to Docker for production)
    SandboxConfig sandbox;
};

class RendererRegistry {
public:
    void register_adapter(std::unique_ptr<RendererAdapter> adapter);
    const RendererAdapter* get(std::string_view name) const;
    std::vector<std::string> list() const;

private:
    std::unordered_map<std::string, std::unique_ptr<RendererAdapter>> adapters_;
};

// ─── Built-in adapters ───────────────────────────────────────────────────────

// Pure C++ SVG renderer — no external binary required. Always available.
class SvgAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
};

// FFmpeg: composition and encoding. Converts SVG frames to MP4/WebM/GIF.
class FfmpegAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
};

// MermaidJS: renders Diagram nodes with language=mermaid via the `mmdc` CLI.
class MermaidAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
};

// PlantUML: renders Diagram nodes with language=plant_uml via the PlantUML server API.
// No local Java runtime required — uses https://www.plantuml.com/plantuml (or a self-hosted server).
class PlantUmlAdapter final : public RendererAdapter {
public:
    explicit PlantUmlAdapter(std::string server_url = "https://www.plantuml.com/plantuml");
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
private:
    std::string server_url_;
};

// Manim: renders math/animation scenes via the `manim` Python CLI.
class ManimAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
};

// Remotion: renders web-native animation scenes via `npx remotion render`.
class RemotionAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene&, const RenderSettings&, const std::filesystem::path&) const override;
    RenderArtifact execute(const RenderPlan&) const override;
    HealthStatus health_check() const override;
};

}  // namespace openanim
