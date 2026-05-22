#pragma once

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
    virtual RenderPlan compile(const Scene& scene, const RenderSettings& settings, const std::filesystem::path& output_dir) const = 0;
    virtual RenderArtifact execute(const RenderPlan& plan) const = 0;
    virtual HealthStatus health_check() const = 0;
};

class RendererRegistry {
public:
    void register_adapter(std::unique_ptr<RendererAdapter> adapter);
    const RendererAdapter* get(std::string_view name) const;
    std::vector<std::string> list() const;

private:
    std::unordered_map<std::string, std::unique_ptr<RendererAdapter>> adapters_;
};

class SvgAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene& scene, const RenderSettings& settings, const std::filesystem::path& output_dir) const override;
    RenderArtifact execute(const RenderPlan& plan) const override;
    HealthStatus health_check() const override;
};

class FfmpegAdapter final : public RendererAdapter {
public:
    std::string name() const override;
    std::string version() const override;
    std::vector<NodeType> supported_node_types() const override;
    RenderPlan compile(const Scene& scene, const RenderSettings& settings, const std::filesystem::path& output_dir) const override;
    RenderArtifact execute(const RenderPlan& plan) const override;
    HealthStatus health_check() const override;
};

}  // namespace openanim
