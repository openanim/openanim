#include "openanim/renderer.hpp"

#include <algorithm>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <stdexcept>

namespace openanim {
namespace {

std::string escape_xml(std::string_view text) {
    std::string out;
    for (char c : text) {
        switch (c) {
            case '&': out += "&amp;"; break;
            case '<': out += "&lt;"; break;
            case '>': out += "&gt;"; break;
            case '"': out += "&quot;"; break;
            default: out.push_back(c); break;
        }
    }
    return out;
}

std::string color_to_svg(const Color& color) {
    auto channel = [](double value) {
        auto scaled = std::clamp(static_cast<int>(std::round(value * 255.0)), 0, 255);
        std::ostringstream out;
        out << std::hex << std::setw(2) << std::setfill('0') << scaled;
        return out.str();
    };
    return "#" + channel(color.r) + channel(color.g) + channel(color.b);
}

Vec2 node_position(const Node& node) {
    if (node.components.transform) {
        return node.components.transform->position;
    }
    return {};
}

std::string node_style(const Node& node) {
    std::string fill = "none";
    std::string stroke = "none";
    double stroke_width = 1.0;
    double opacity = 1.0;

    if (node.components.style) {
        const auto& style = *node.components.style;
        if (!style.visible) return "display=\"none\"";
        if (style.fill) fill = color_to_svg(*style.fill);
        if (style.stroke) {
            stroke = color_to_svg(style.stroke->color);
            stroke_width = style.stroke->width;
        }
        opacity = style.opacity;
    }

    std::ostringstream out;
    out << "fill=\"" << fill << "\" stroke=\"" << stroke << "\" stroke-width=\"" << stroke_width << "\" opacity=\"" << opacity << "\"";
    return out.str();
}

std::string render_shape(const Node& node) {
    const auto& shape = *node.components.shape;
    const auto& k = shape.kind;
    const auto p = node_position(node);
    std::ostringstream out;

    switch (k.type) {
        case ShapeType::Rectangle:
            out << "<rect x=\"" << p.x << "\" y=\"" << p.y << "\" width=\"" << k.width << "\" height=\"" << k.height << "\" rx=\"" << k.corner_radius << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Circle:
            out << "<circle cx=\"" << p.x << "\" cy=\"" << p.y << "\" r=\"" << k.radius << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Ellipse:
            out << "<ellipse cx=\"" << p.x << "\" cy=\"" << p.y << "\" rx=\"" << k.rx << "\" ry=\"" << k.ry << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Line:
            out << "<line x1=\"" << p.x + k.start.x << "\" y1=\"" << p.y + k.start.y << "\" x2=\"" << p.x + k.end.x << "\" y2=\"" << p.y + k.end.y << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Polygon:
            out << (k.closed ? "<polygon points=\"" : "<polyline points=\"");
            for (const auto& point : k.points) out << p.x + point.x << "," << p.y + point.y << " ";
            out << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Path:
            out << "<path d=\"" << escape_xml(k.data) << "\" transform=\"translate(" << p.x << " " << p.y << ")\" " << node_style(node) << "/>";
            break;
        case ShapeType::Arc:
        case ShapeType::Arrow:
            out << "<line x1=\"" << p.x + k.start.x << "\" y1=\"" << p.y + k.start.y << "\" x2=\"" << p.x + k.end.x << "\" y2=\"" << p.y + k.end.y << "\" " << node_style(node) << "/>";
            break;
    }
    return out.str();
}

std::string render_text(const Node& node) {
    const auto& text = *node.components.text;
    const auto p = node_position(node);
    std::string fill = "#ffffff";
    double opacity = 1.0;
    if (node.components.style) {
        if (node.components.style->fill) fill = color_to_svg(*node.components.style->fill);
        opacity = node.components.style->opacity;
    }
    std::string anchor = text.align == TextAlign::Center ? "middle" : text.align == TextAlign::Right ? "end" : "start";
    std::ostringstream out;
    out << "<text x=\"" << p.x << "\" y=\"" << p.y << "\" font-family=\"" << escape_xml(text.font.family) << "\" font-size=\"" << text.font.size << "\" text-anchor=\"" << anchor << "\" fill=\"" << fill << "\" opacity=\"" << opacity << "\">" << escape_xml(text.content) << "</text>";
    return out.str();
}

std::string scene_to_svg(const Scene& scene, const RenderSettings& settings) {
    std::ostringstream out;
    out << "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"" << settings.resolution.first << "\" height=\"" << settings.resolution.second << "\" viewBox=\"0 0 " << settings.resolution.first << " " << settings.resolution.second << "\">";
    out << "<rect width=\"100%\" height=\"100%\" fill=\"" << color_to_svg(settings.background_color) << "\"/>";
    for (const auto& node : scene.nodes) {
        if (node.node_type == NodeType::Shape && node.components.shape) {
            out << render_shape(node);
        } else if (node.node_type == NodeType::Text && node.components.text) {
            out << render_text(node);
        } else if (node.node_type == NodeType::Image && node.components.image) {
            const auto p = node_position(node);
            const auto& image = *node.components.image;
            out << "<image x=\"" << p.x << "\" y=\"" << p.y << "\" href=\"" << escape_xml(image.asset_ref.path) << "\"";
            if (image.width) out << " width=\"" << *image.width << "\"";
            if (image.height) out << " height=\"" << *image.height << "\"";
            out << "/>";
        }
    }
    out << "</svg>";
    return out.str();
}

std::string extension_for(OutputFormat format) {
    return format == OutputFormat::Svg ? "svg" : to_string(format);
}

}  // namespace

void RendererRegistry::register_adapter(std::unique_ptr<RendererAdapter> adapter) {
    adapters_[adapter->name()] = std::move(adapter);
}

const RendererAdapter* RendererRegistry::get(std::string_view name) const {
    auto it = adapters_.find(std::string{name});
    return it == adapters_.end() ? nullptr : it->second.get();
}

std::vector<std::string> RendererRegistry::list() const {
    std::vector<std::string> names;
    for (const auto& [name, _] : adapters_) names.push_back(name);
    std::sort(names.begin(), names.end());
    return names;
}

std::string SvgAdapter::name() const { return "svg"; }
std::string SvgAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> SvgAdapter::supported_node_types() const {
    return {NodeType::Group, NodeType::Shape, NodeType::Text, NodeType::Image};
}

RenderPlan SvgAdapter::compile(const Scene& scene, const RenderSettings& settings, const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Svg;
    plan.output_path = output_dir / (scene.id.value + "." + extension_for(plan.output_format));
    plan.inline_svg = scene_to_svg(scene, settings);
    return plan;
}

RenderArtifact SvgAdapter::execute(const RenderPlan& plan) const {
    const auto start = std::chrono::steady_clock::now();
    std::filesystem::create_directories(plan.output_path.parent_path());
    std::ofstream file(plan.output_path);
    if (!file || !plan.inline_svg) {
        throw std::runtime_error("failed to write SVG artifact");
    }
    file << *plan.inline_svg;
    file.close();

    RenderArtifact artifact;
    artifact.plan_id = plan.id;
    artifact.output_path = plan.output_path;
    artifact.format = OutputFormat::Svg;
    artifact.file_size_bytes = std::filesystem::file_size(plan.output_path);
    artifact.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start);
    artifact.stdout_text = "Wrote SVG artifact";
    return artifact;
}

HealthStatus SvgAdapter::health_check() const {
    return {true, version(), "Built-in SVG renderer ready"};
}

std::string FfmpegAdapter::name() const { return "ffmpeg"; }
std::string FfmpegAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> FfmpegAdapter::supported_node_types() const {
    return {NodeType::Group, NodeType::Shape, NodeType::Text, NodeType::Image};
}

RenderPlan FfmpegAdapter::compile(const Scene& scene, const RenderSettings& settings, const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Svg;
    plan.output_path = output_dir / (scene.id.value + ".svg");
    plan.inline_svg = scene_to_svg(scene, settings);
    return plan;
}

RenderArtifact FfmpegAdapter::execute(const RenderPlan& plan) const {
    SvgAdapter svg;
    auto artifact = svg.execute(plan);
    artifact.stdout_text = "FFmpeg adapter prototype emitted SVG fallback artifact";
    return artifact;
}

HealthStatus FfmpegAdapter::health_check() const {
    return {true, version(), "Prototype FFmpeg adapter using SVG fallback"};
}

}  // namespace openanim
