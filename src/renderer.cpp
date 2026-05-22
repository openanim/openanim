#include "openanim/renderer.hpp"
#include "openanim/sandbox.hpp"

#include <algorithm>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <stdexcept>

// Forward declaration from llm.cpp (used only by PlantUmlAdapter)
namespace openanim { std::string llm_http_get(const std::string& url); }

namespace openanim {
namespace {

// ─── SVG helpers (used by SVG and FFmpeg adapters) ───────────────────────────

std::string escape_xml(std::string_view text) {
    std::string out;
    for (char c : text) {
        switch (c) {
            case '&': out += "&amp;"; break;
            case '<': out += "&lt;";  break;
            case '>': out += "&gt;";  break;
            case '"': out += "&quot;"; break;
            default:  out.push_back(c); break;
        }
    }
    return out;
}

std::string color_to_svg(const Color& color) {
    auto ch = [](double v) {
        std::ostringstream o;
        o << std::hex << std::setw(2) << std::setfill('0')
          << std::clamp(static_cast<int>(std::round(v * 255.0)), 0, 255);
        return o.str();
    };
    return "#" + ch(color.r) + ch(color.g) + ch(color.b);
}

Vec2 node_position(const Node& node) {
    return node.components.transform ? node.components.transform->position : Vec2{};
}

std::string node_style(const Node& node) {
    std::string fill = "none", stroke = "none";
    double stroke_width = 1.0, opacity = 1.0;
    if (node.components.style) {
        const auto& s = *node.components.style;
        if (!s.visible) return "display=\"none\"";
        if (s.fill)   fill   = color_to_svg(*s.fill);
        if (s.stroke) { stroke = color_to_svg(s.stroke->color); stroke_width = s.stroke->width; }
        opacity = s.opacity;
    }
    std::ostringstream o;
    o << "fill=\"" << fill << "\" stroke=\"" << stroke
      << "\" stroke-width=\"" << stroke_width << "\" opacity=\"" << opacity << "\"";
    return o.str();
}

std::string render_shape_svg(const Node& node) {
    const auto& k = node.components.shape->kind;
    const auto p = node_position(node);
    std::ostringstream o;
    switch (k.type) {
        case ShapeType::Rectangle:
            o << "<rect x=\"" << p.x << "\" y=\"" << p.y << "\" width=\"" << k.width
              << "\" height=\"" << k.height << "\" rx=\"" << k.corner_radius << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Circle:
            o << "<circle cx=\"" << p.x << "\" cy=\"" << p.y << "\" r=\"" << k.radius << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Ellipse:
            o << "<ellipse cx=\"" << p.x << "\" cy=\"" << p.y << "\" rx=\"" << k.rx << "\" ry=\"" << k.ry << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Line:
            o << "<line x1=\"" << p.x + k.start.x << "\" y1=\"" << p.y + k.start.y
              << "\" x2=\"" << p.x + k.end.x << "\" y2=\"" << p.y + k.end.y << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Polygon:
            o << (k.closed ? "<polygon points=\"" : "<polyline points=\"");
            for (const auto& pt : k.points) o << p.x + pt.x << "," << p.y + pt.y << " ";
            o << "\" " << node_style(node) << "/>";
            break;
        case ShapeType::Path:
            o << "<path d=\"" << escape_xml(k.data) << "\" transform=\"translate(" << p.x << " " << p.y << ")\" " << node_style(node) << "/>";
            break;
        default:
            o << "<line x1=\"" << p.x + k.start.x << "\" y1=\"" << p.y + k.start.y
              << "\" x2=\"" << p.x + k.end.x << "\" y2=\"" << p.y + k.end.y << "\" " << node_style(node) << "/>";
    }
    return o.str();
}

std::string render_text_svg(const Node& node) {
    const auto& t = *node.components.text;
    const auto p = node_position(node);
    std::string fill = "#ffffff";
    double opacity = 1.0;
    if (node.components.style) {
        if (node.components.style->fill) fill = color_to_svg(*node.components.style->fill);
        opacity = node.components.style->opacity;
    }
    std::string anchor = (t.align == TextAlign::Center) ? "middle" : (t.align == TextAlign::Right) ? "end" : "start";
    std::ostringstream o;
    o << "<text x=\"" << p.x << "\" y=\"" << p.y
      << "\" font-family=\"" << escape_xml(t.font.family)
      << "\" font-size=\"" << t.font.size
      << "\" text-anchor=\"" << anchor
      << "\" fill=\"" << fill << "\" opacity=\"" << opacity << "\">"
      << escape_xml(t.content) << "</text>";
    return o.str();
}

std::string scene_to_svg(const Scene& scene, const RenderSettings& settings) {
    std::ostringstream o;
    o << "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"" << settings.resolution.first
      << "\" height=\"" << settings.resolution.second
      << "\" viewBox=\"0 0 " << settings.resolution.first << " " << settings.resolution.second << "\">";
    o << "<rect width=\"100%\" height=\"100%\" fill=\"" << color_to_svg(settings.background_color) << "\"/>";
    for (const auto& node : scene.nodes) {
        if (node.node_type == NodeType::Shape && node.components.shape) o << render_shape_svg(node);
        else if (node.node_type == NodeType::Text && node.components.text) o << render_text_svg(node);
        else if (node.node_type == NodeType::Image && node.components.image) {
            const auto p = node_position(node);
            const auto& img = *node.components.image;
            o << "<image x=\"" << p.x << "\" y=\"" << p.y << "\" href=\"" << escape_xml(img.asset_ref.path) << "\"";
            if (img.width)  o << " width=\""  << *img.width  << "\"";
            if (img.height) o << " height=\"" << *img.height << "\"";
            o << "/>";
        }
    }
    o << "</svg>";
    return o.str();
}


// ─── PlantUML hex encoding (~h prefix for server API) ────────────────────────

std::string plantuml_hex_encode(const std::string& source) {
    std::ostringstream hex;
    hex << std::hex << std::setfill('0');
    for (unsigned char c : source) hex << std::setw(2) << static_cast<int>(c);
    return "~h" + hex.str();
}

// ─── Manim code generator ─────────────────────────────────────────────────────
// Translates Scene IR → Manim Community Edition Python script

static double px_to_mx(double px, uint32_t w) { return (px - w / 2.0) / (w / 2.0) * 7.111; }
static double px_to_my(double py, uint32_t h) { return -(py - h / 2.0) / (h / 2.0) * 4.0; }
static double px_to_ms(double sz, uint32_t w)  { return sz / (w / 2.0) * 7.111; }

static std::string manim_var(const Id& id) {
    std::string out = "obj_";
    for (char c : id.value) if (std::isalnum(c)) out += c;
    return out.substr(0, 20);
}

static std::string manim_rate(EasingFunction ef) {
    switch (ef) {
        case EasingFunction::Linear:    return "linear";
        case EasingFunction::EaseIn:    return "rush_into";
        case EasingFunction::EaseOut:   return "rush_from";
        default:                        return "smooth";
    }
}

static std::string scene_to_manim_py(const Scene& scene, const RenderSettings& settings) {
    const auto W = settings.resolution.first;
    const auto H = settings.resolution.second;
    std::ostringstream py;
    py << "from manim import *\nimport numpy as np\n\n";
    py << "class GeneratedScene(Scene):\n";
    py << "    def construct(self):\n";
    py << "        self.camera.background_color = \"" << color_to_svg(settings.background_color) << "\"\n\n";

    // Collect renderable nodes
    std::map<std::string, std::string> var_of;
    for (const auto& node : scene.nodes) {
        if (!node.is_renderable()) continue;
        const std::string var = manim_var(node.id);
        var_of[node.id.value] = var;
        Vec2 pos = node.components.transform ? node.components.transform->position : Vec2{(double)W/2,(double)H/2};
        double mx = px_to_mx(pos.x, W), my = px_to_my(pos.y, H);

        std::string fill = "#ffffff";
        double fill_op = 1.0, stroke_w = 0.0, op = 1.0;
        std::string stroke_col = "#ffffff";
        bool visible = true;
        if (node.components.style) {
            const auto& s = *node.components.style;
            visible = s.visible;
            op = s.opacity;
            if (s.fill) { fill = color_to_svg(*s.fill); fill_op = s.fill->a; }
            if (s.stroke) { stroke_col = color_to_svg(s.stroke->color); stroke_w = px_to_ms(s.stroke->width, W); }
        }
        if (!visible) continue;

        if (node.components.shape) {
            const auto& k = node.components.shape->kind;
            switch (k.type) {
                case ShapeType::Circle:
                    py << "        " << var << " = Circle(radius=" << px_to_ms(k.radius, W) << ")\n";
                    break;
                case ShapeType::Rectangle:
                    py << "        " << var << " = Rectangle(width=" << px_to_ms(k.width, W)
                       << ", height=" << px_to_ms(k.height, H) << ")\n";
                    break;
                case ShapeType::Line:
                    py << "        " << var << " = Line(start=np.array(["
                       << px_to_mx(pos.x+k.start.x, W) << ", " << px_to_my(pos.y+k.start.y, H) << ", 0]), "
                       << "end=np.array([" << px_to_mx(pos.x+k.end.x, W) << ", " << px_to_my(pos.y+k.end.y, H) << ", 0]))\n";
                    break;
                case ShapeType::Ellipse:
                    py << "        " << var << " = Ellipse(width=" << px_to_ms(k.rx*2, W)
                       << ", height=" << px_to_ms(k.ry*2, H) << ")\n";
                    break;
                default:
                    py << "        " << var << " = Circle(radius=0.5)  # unsupported shape\n";
            }
            py << "        " << var << ".set_fill(color=\"" << fill << "\", opacity=" << fill_op << ")\n";
            py << "        " << var << ".set_stroke(color=\"" << stroke_col << "\", width=" << stroke_w << ")\n";
            py << "        " << var << ".set_opacity(" << op << ")\n";
            py << "        " << var << ".move_to(np.array([" << mx << ", " << my << ", 0]))\n\n";
        } else if (node.components.text) {
            const auto& t = *node.components.text;
            py << "        " << var << " = Text(r\"" << t.content << "\", font_size=" << (int)(t.font.size * 1.5) << ")\n";
            py << "        " << var << ".set_color(\"" << fill << "\")\n";
            py << "        " << var << ".set_opacity(" << op << ")\n";
            py << "        " << var << ".move_to(np.array([" << mx << ", " << my << ", 0]))\n\n";
        } else if (node.components.math) {
            const auto& m = *node.components.math;
            py << "        " << var << " = MathTex(r\"" << m.latex << "\", font_size=" << (int)m.font_size << ")\n";
            py << "        " << var << ".set_color(\"" << color_to_svg(m.color) << "\")\n";
            py << "        " << var << ".move_to(np.array([" << mx << ", " << my << ", 0]))\n\n";
        }
    }

    // Add all objects
    py << "        # Initial state\n        self.add(";
    bool first = true;
    for (const auto& [id, var] : var_of) { if (!first) py << ", "; py << var; first = false; }
    py << ")\n\n";

    // Timeline keyframe tracks
    struct AnimMoment { double t; double dur; std::string code; };
    std::vector<AnimMoment> moments;

    for (const auto& track : scene.timeline.tracks) {
        auto it = var_of.find(track.target_node.value);
        if (it == var_of.end()) continue;
        const auto& var = it->second;
        const auto& kfs = track.keyframes;
        for (size_t i = 0; i + 1 < kfs.size(); ++i) {
            double dur = kfs[i+1].time.value - kfs[i].time.value;
            if (dur <= 0) continue;
            std::string rate = manim_rate(kfs[i+1].easing);
            std::ostringstream anim;
            switch (track.property) {
                case AnimatableProperty::Opacity: {
                    double v = std::holds_alternative<double>(kfs[i+1].value) ? std::get<double>(kfs[i+1].value) : 1.0;
                    anim << "self.play(" << var << ".animate.set_opacity(" << v << "), run_time=" << dur << ", rate_func=" << rate << ")";
                    break;
                }
                case AnimatableProperty::Position:
                    if (std::holds_alternative<Vec2>(kfs[i+1].value)) {
                        const auto& v = std::get<Vec2>(kfs[i+1].value);
                        anim << "self.play(" << var << ".animate.move_to(np.array([" << px_to_mx(v.x,W) << ", " << px_to_my(v.y,H) << ", 0])), run_time=" << dur << ", rate_func=" << rate << ")";
                    }
                    break;
                case AnimatableProperty::PositionX:
                    if (std::holds_alternative<double>(kfs[i+1].value))
                        anim << "self.play(" << var << ".animate.set_x(" << px_to_mx(std::get<double>(kfs[i+1].value),W) << "), run_time=" << dur << ", rate_func=" << rate << ")";
                    break;
                case AnimatableProperty::PositionY:
                    if (std::holds_alternative<double>(kfs[i+1].value))
                        anim << "self.play(" << var << ".animate.set_y(" << px_to_my(std::get<double>(kfs[i+1].value),H) << "), run_time=" << dur << ", rate_func=" << rate << ")";
                    break;
                case AnimatableProperty::Rotation:
                    if (std::holds_alternative<double>(kfs[i+1].value))
                        anim << "self.play(" << var << ".animate.rotate(" << std::get<double>(kfs[i+1].value) << "), run_time=" << dur << ", rate_func=" << rate << ")";
                    break;
                case AnimatableProperty::Scale:
                    if (std::holds_alternative<double>(kfs[i+1].value))
                        anim << "self.play(" << var << ".animate.scale(" << std::get<double>(kfs[i+1].value) << "), run_time=" << dur << ", rate_func=" << rate << ")";
                    break;
                default: break;
            }
            if (!anim.str().empty()) moments.push_back({kfs[i].time.value, dur, anim.str()});
        }
    }

    // Timeline events
    for (const auto* ev : scene.timeline.sorted_events()) {
        for (const auto& tid : ev->target_nodes) {
            auto it = var_of.find(tid.value);
            if (it == var_of.end()) continue;
            const auto& var = it->second;
            std::string anim;
            double dur = ev->duration.value > 0 ? ev->duration.value : 1.0;
            switch (ev->event_type) {
                case EventType::FadeIn:  anim = "self.play(FadeIn("  + var + "), run_time=" + std::to_string(dur) + ")"; break;
                case EventType::FadeOut: anim = "self.play(FadeOut(" + var + "), run_time=" + std::to_string(dur) + ")"; break;
                case EventType::Create:  anim = "self.play(Create("  + var + "), run_time=" + std::to_string(dur) + ")"; break;
                case EventType::Write:   anim = "self.play(Write("   + var + "), run_time=" + std::to_string(dur) + ")"; break;
                default: break;
            }
            if (!anim.empty()) moments.push_back({ev->start_time.value, dur, anim});
        }
    }

    std::sort(moments.begin(), moments.end(), [](const AnimMoment& a, const AnimMoment& b){ return a.t < b.t; });

    if (!moments.empty()) {
        py << "        # Animations\n";
        double cur = 0.0;
        for (const auto& m : moments) {
            if (m.t - cur > 0.01) py << "        self.wait(" << (m.t - cur) << ")\n";
            py << "        " << m.code << "\n";
            cur = m.t + m.dur;
        }
    }

    py << "\n        self.wait(0.5)\n";
    return py.str();
}

// ─── Remotion code generator ──────────────────────────────────────────────────
// Translates Scene IR → React/TypeScript Remotion composition

static std::string css_rgba(const Color& c) {
    std::ostringstream o;
    o << "rgba(" << (int)(c.r*255) << "," << (int)(c.g*255) << "," << (int)(c.b*255) << "," << c.a << ")";
    return o.str();
}

static std::string scene_to_remotion_tsx(const Scene& scene, const RenderSettings& settings) {
    const auto W = settings.resolution.first;
    const auto H = settings.resolution.second;
    const double fps = settings.fps;
    std::ostringstream tsx;
    tsx << "import React from 'react';\n";
    tsx << "import {useCurrentFrame, useVideoConfig, interpolate, AbsoluteFill} from 'remotion';\n\n";
    tsx << "export const GeneratedScene: React.FC = () => {\n";
    tsx << "  const frame = useCurrentFrame();\n";
    tsx << "  const {fps: _fps} = useVideoConfig();\n\n";

    // Build interpolated variables for timeline tracks
    for (const auto& track : scene.timeline.tracks) {
        if (track.keyframes.size() < 2) continue;
        std::string id = track.target_node.value;
        id.erase(std::remove(id.begin(), id.end(), '-'), id.end());
        std::string prop = to_string(track.property);
        prop.erase(std::remove(prop.begin(), prop.end(), '_'), prop.end());
        std::string vname = "v_" + id.substr(0, 12) + "_" + prop;

        std::ostringstream frames_arr, vals_arr;
        frames_arr << "[";
        vals_arr << "[";
        for (size_t i = 0; i < track.keyframes.size(); ++i) {
            const auto& kf = track.keyframes[i];
            if (i) { frames_arr << ","; vals_arr << ","; }
            frames_arr << (int)(kf.time.value * fps);
            double val = 0.0;
            if (std::holds_alternative<double>(kf.value)) val = std::get<double>(kf.value);
            else if (std::holds_alternative<Vec2>(kf.value)) val = std::get<Vec2>(kf.value).x;
            vals_arr << val;
        }
        frames_arr << "]";
        vals_arr << "]";
        tsx << "  const " << vname << " = interpolate(frame, " << frames_arr.str() << ", " << vals_arr.str()
            << ", {extrapolateLeft:'clamp', extrapolateRight:'clamp'});\n";
    }
    tsx << "\n";
    tsx << "  return (\n";
    tsx << "    <AbsoluteFill style={{backgroundColor:'" << css_rgba(settings.background_color) << "'}}>\n";

    // Render nodes
    for (const auto& node : scene.nodes) {
        if (!node.is_renderable()) continue;
        Vec2 pos = node.components.transform ? node.components.transform->position : Vec2{(double)W/2,(double)H/2};
        std::string fill = "#ffffff";
        double op = 1.0;
        if (node.components.style) {
            const auto& s = *node.components.style;
            if (s.fill) fill = color_to_svg(*s.fill);
            op = s.opacity;
        }

        // Check if position is animated
        std::string id_clean = node.id.value;
        id_clean.erase(std::remove(id_clean.begin(), id_clean.end(), '-'), id_clean.end());
        std::string px_var, py_var, op_var;
        for (const auto& track : scene.timeline.tracks) {
            if (track.target_node.value != node.id.value) continue;
            std::string prop = to_string(track.property);
            prop.erase(std::remove(prop.begin(), prop.end(), '_'), prop.end());
            std::string vname = "v_" + id_clean.substr(0, 12) + "_" + prop;
            if (track.property == AnimatableProperty::PositionX) px_var = vname;
            if (track.property == AnimatableProperty::PositionY) py_var = vname;
            if (track.property == AnimatableProperty::Opacity)   op_var = vname;
        }
        std::string sx = px_var.empty() ? std::to_string((int)pos.x) : px_var;
        std::string sy = py_var.empty() ? std::to_string((int)pos.y) : py_var;
        std::string sop = op_var.empty() ? std::to_string(op) : op_var;

        if (node.components.shape) {
            const auto& k = node.components.shape->kind;
            switch (k.type) {
                case ShapeType::Circle:
                    tsx << "      <div style={{position:'absolute',left:" << sx << "-" << k.radius
                        << ",top:" << sy << "-" << k.radius << ",width:" << k.radius*2 << ",height:" << k.radius*2
                        << ",borderRadius:'50%',backgroundColor:'" << fill << "',opacity:" << sop << "}} />\n";
                    break;
                case ShapeType::Rectangle:
                    tsx << "      <div style={{position:'absolute',left:" << sx << "-" << k.width/2
                        << ",top:" << sy << "-" << k.height/2 << ",width:" << k.width << ",height:" << k.height
                        << ",borderRadius:" << k.corner_radius << ",backgroundColor:'" << fill << "',opacity:" << sop << "}} />\n";
                    break;
                default:
                    tsx << "      {/* unsupported shape */}\n";
            }
        } else if (node.components.text) {
            const auto& t = *node.components.text;
            tsx << "      <div style={{position:'absolute',left:" << sx << ",top:" << sy
                << ",fontFamily:'" << t.font.family << "',fontSize:" << (int)t.font.size
                << ",color:'" << fill << "',opacity:" << sop << "}}>"
                << t.content << "</div>\n";
        }
    }
    tsx << "    </AbsoluteFill>\n  );\n};\n";
    return tsx.str();
}


}  // namespace


// ─── RendererRegistry ─────────────────────────────────────────────────────────

void RendererRegistry::register_adapter(std::unique_ptr<RendererAdapter> adapter) {
    adapters_[adapter->name()] = std::move(adapter);
}
const RendererAdapter* RendererRegistry::get(std::string_view name) const {
    auto it = adapters_.find(std::string{name});
    return it == adapters_.end() ? nullptr : it->second.get();
}
std::vector<std::string> RendererRegistry::list() const {
    std::vector<std::string> names;
    for (const auto& [n, _] : adapters_) names.push_back(n);
    std::sort(names.begin(), names.end());
    return names;
}

// ─── SvgAdapter ──────────────────────────────────────────────────────────────

std::string SvgAdapter::name()    const { return "svg"; }
std::string SvgAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> SvgAdapter::supported_node_types() const {
    return {NodeType::Group, NodeType::Shape, NodeType::Text, NodeType::Image};
}
RenderPlan SvgAdapter::compile(const Scene& scene, const RenderSettings& settings,
                               const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Svg;
    plan.output_path = output_dir / (scene.id.value + ".svg");
    plan.inline_svg = scene_to_svg(scene, settings);
    return plan;
}
RenderArtifact SvgAdapter::execute(const RenderPlan& plan) const {
    const auto start = std::chrono::steady_clock::now();
    std::filesystem::create_directories(plan.output_path.parent_path());
    std::ofstream file(plan.output_path);
    if (!file || !plan.inline_svg) throw std::runtime_error("failed to write SVG artifact");
    file << *plan.inline_svg;
    file.close();
    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = plan.output_path;
    art.format = OutputFormat::Svg;
    art.file_size_bytes = std::filesystem::file_size(plan.output_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    art.stdout_text = "SVG written";
    return art;
}
HealthStatus SvgAdapter::health_check() const {
    return {true, version(), "Built-in SVG renderer — always available"};
}

// ─── FfmpegAdapter ────────────────────────────────────────────────────────────
// Converts SVG frames to MP4 using `ffmpeg -loop 1 -i input.svg ...`

std::string FfmpegAdapter::name()    const { return "ffmpeg"; }
std::string FfmpegAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> FfmpegAdapter::supported_node_types() const {
    return {NodeType::Group, NodeType::Shape, NodeType::Text, NodeType::Image};
}
RenderPlan FfmpegAdapter::compile(const Scene& scene, const RenderSettings& settings,
                                  const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Mp4;
    plan.output_path = output_dir / (scene.id.value + ".mp4");
    plan.inline_svg = scene_to_svg(scene, settings);
    return plan;
}
RenderArtifact FfmpegAdapter::execute(const RenderPlan& plan) const {
    const auto start = std::chrono::steady_clock::now();
    if (!plan.inline_svg) throw std::runtime_error("FFmpeg adapter requires SVG input");

    std::filesystem::create_directories(plan.output_path.parent_path());

    // Try to rasterize SVG→PNG first (rsvg-convert or convert/ImageMagick)
    auto svg_path = plan.output_path.parent_path() / (plan.id.value + "_input.svg");
    auto png_path = plan.output_path.parent_path() / (plan.id.value + "_frame.png");
    { std::ofstream f(svg_path); f << *plan.inline_svg; }

    bool has_png = false;
    // Try rsvg-convert (brew install librsvg)
    if (binary_exists("rsvg-convert")) {
        auto r = run_command("rsvg-convert", {
            "-w", "1920", "-h", "1080",
            "-o", png_path.string(),
            svg_path.string()
        }, plan.output_path.parent_path(), sandbox);
        has_png = (r.exit_code == 0 && std::filesystem::exists(png_path));
    }
    // Fallback: ImageMagick convert
    if (!has_png && binary_exists("convert")) {
        auto r = run_command("convert", {
            "-background", "none",
            "-resize", "1920x1080",
            svg_path.string(), png_path.string()
        }, plan.output_path.parent_path(), sandbox);
        has_png = (r.exit_code == 0 && std::filesystem::exists(png_path));
    }

    SubprocessResult result;
    auto mp4_path = plan.output_path;

    if (has_png) {
        // Encode still-image PNG → MP4
        result = run_command("ffmpeg", {
            "-y", "-loop", "1",
            "-i", png_path.string(),
            "-t", "5",
            "-r", "30",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            mp4_path.string()
        }, plan.output_path.parent_path(), sandbox);
    } else {
        // Last resort: solid-color lavfi source (no image required)
        result = run_command("ffmpeg", {
            "-y",
            "-f", "lavfi",
            "-i", "color=black:s=1920x1080:r=30:d=5",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            mp4_path.string()
        }, plan.output_path.parent_path(), sandbox);
    }

    std::filesystem::remove(svg_path);
    if (has_png) std::filesystem::remove(png_path);

    if (result.exit_code != 0) {
        throw std::runtime_error("FFmpeg failed (exit " + std::to_string(result.exit_code) + "): " + result.stderr_text);
    }

    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = mp4_path;
    art.format = OutputFormat::Mp4;
    art.file_size_bytes = std::filesystem::file_size(mp4_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    art.stdout_text = result.stdout_text;
    art.stderr_text = result.stderr_text;
    return art;
}
HealthStatus FfmpegAdapter::health_check() const {
    bool ok = binary_exists("ffmpeg");
    return {ok, {}, ok ? "ffmpeg found on PATH" : "ffmpeg not found — install via `brew install ffmpeg`"};
}

// ─── MermaidAdapter ───────────────────────────────────────────────────────────
// Extracts Diagram source from the scene, writes a .mmd file, runs `mmdc`

std::string MermaidAdapter::name()    const { return "mermaid"; }
std::string MermaidAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> MermaidAdapter::supported_node_types() const { return {NodeType::Diagram}; }

RenderPlan MermaidAdapter::compile(const Scene& scene, const RenderSettings& /*settings*/,
                                   const std::filesystem::path& output_dir) const {
    // Collect all Mermaid diagram sources from the scene
    std::ostringstream combined;
    for (const auto& node : scene.nodes) {
        if (node.components.diagram &&
            node.components.diagram->language == DiagramLanguage::Mermaid) {
            combined << node.components.diagram->source << "\n";
        }
    }
    if (combined.str().empty()) {
        throw std::runtime_error("MermaidAdapter: scene contains no Mermaid diagram nodes");
    }
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Svg;
    plan.output_path = output_dir / (scene.id.value + ".svg");
    plan.source_text = combined.str();
    return plan;
}

RenderArtifact MermaidAdapter::execute(const RenderPlan& plan) const {
    const auto start = std::chrono::steady_clock::now();
    if (!plan.source_text) throw std::runtime_error("MermaidAdapter: no source_text in plan");

    std::filesystem::create_directories(plan.output_path.parent_path());
    auto mmd_path = plan.output_path.parent_path() / (plan.id.value + ".mmd");
    { std::ofstream f(mmd_path); f << *plan.source_text; }

    auto result = run_command("mmdc", {
        "-i", mmd_path.filename().string(),
        "-o", plan.output_path.filename().string()
    }, plan.output_path.parent_path(), sandbox);

    std::filesystem::remove(mmd_path);

    if (result.exit_code != 0) {
        throw std::runtime_error("mmdc failed (exit " + std::to_string(result.exit_code) + "): " + result.stderr_text);
    }

    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = plan.output_path;
    art.format = OutputFormat::Svg;
    art.file_size_bytes = std::filesystem::file_size(plan.output_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    art.stdout_text = result.stdout_text;
    return art;
}
HealthStatus MermaidAdapter::health_check() const {
    bool ok = binary_exists("mmdc");
    return {ok, {}, ok ? "mmdc found on PATH" : "mmdc not found — install via `npm install -g @mermaid-js/mermaid-cli`"};
}

// ─── PlantUmlAdapter ─────────────────────────────────────────────────────────
// Uses PlantUML server API with ~h hex encoding — no Java required

PlantUmlAdapter::PlantUmlAdapter(std::string server_url) : server_url_(std::move(server_url)) {}
std::string PlantUmlAdapter::name()    const { return "plantuml"; }
std::string PlantUmlAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> PlantUmlAdapter::supported_node_types() const { return {NodeType::Diagram}; }

RenderPlan PlantUmlAdapter::compile(const Scene& scene, const RenderSettings& /*settings*/,
                                    const std::filesystem::path& output_dir) const {
    std::ostringstream combined;
    for (const auto& node : scene.nodes) {
        if (node.components.diagram &&
            node.components.diagram->language == DiagramLanguage::PlantUml) {
            combined << node.components.diagram->source << "\n";
        }
    }
    if (combined.str().empty()) {
        throw std::runtime_error("PlantUmlAdapter: scene contains no PlantUML diagram nodes");
    }
    RenderPlan plan;
    plan.provider = name();
    plan.output_format = OutputFormat::Svg;
    plan.output_path = output_dir / (scene.id.value + ".svg");
    plan.source_text = combined.str();
    return plan;
}

RenderArtifact PlantUmlAdapter::execute(const RenderPlan& plan) const {
    const auto start = std::chrono::steady_clock::now();
    if (!plan.source_text) throw std::runtime_error("PlantUmlAdapter: no source_text in plan");

    std::string url = server_url_;
    if (!url.empty() && url.back() == '/') url.pop_back();
    url += "/svg/" + plantuml_hex_encode(*plan.source_text);

    std::string svg = llm_http_get(url);  // reuse curl from llm.cpp

    std::filesystem::create_directories(plan.output_path.parent_path());
    { std::ofstream f(plan.output_path); f << svg; }

    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = plan.output_path;
    art.format = OutputFormat::Svg;
    art.file_size_bytes = std::filesystem::file_size(plan.output_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start);
    art.stdout_text = "PlantUML SVG fetched from server";
    return art;
}
HealthStatus PlantUmlAdapter::health_check() const {
    // Ping the server with a trivial diagram
    try {
        std::string test = "@startuml\nA->B\n@enduml";
        std::string url = server_url_;
        if (!url.empty() && url.back() == '/') url.pop_back();
        url += "/svg/" + plantuml_hex_encode(test);
        llm_http_get(url);
        return {true, {}, "PlantUML server reachable at " + server_url_};
    } catch (const std::exception& e) {
        return {false, {}, std::string("PlantUML server unreachable: ") + e.what()};
    }
}

// ─── ManimAdapter — full implementation ──────────────────────────────────────

std::string ManimAdapter::name()    const { return "manim"; }
std::string ManimAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> ManimAdapter::supported_node_types() const {
    return {NodeType::Math, NodeType::Text, NodeType::Shape};
}
RenderPlan ManimAdapter::compile(const Scene& scene, const RenderSettings& settings,
                                 const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider    = name();
    plan.output_format = OutputFormat::Mp4;
    plan.output_path = output_dir / (scene.id.value + ".mp4");
    plan.source_text = scene_to_manim_py(scene, settings);
    return plan;
}
RenderArtifact ManimAdapter::execute(const RenderPlan& plan) const {
    const auto t0 = std::chrono::steady_clock::now();
    if (!plan.source_text) throw std::runtime_error("ManimAdapter: missing source_text");

    auto work = std::filesystem::temp_directory_path() / ("oa-manim-" + plan.id.value.substr(0,8));
    std::filesystem::create_directories(work);
    auto py_path = work / "scene.py";
    { std::ofstream f(py_path); f << *plan.source_text; }

    auto result = run_command("manim", {
        "render", "scene.py", "GeneratedScene",
        "--output_file", "output",
        "--media_dir", work.string(),
        "--format", "mp4",
        "-q", "m",
        "--disable_caching"
    }, work, sandbox);

    if (result.exit_code != 0) {
        std::filesystem::remove_all(work);
        throw std::runtime_error("manim failed (exit " + std::to_string(result.exit_code) + "):\n" + result.stderr_text);
    }

    // Manim outputs to work/videos/scene/<quality>/output.mp4 — find it
    std::filesystem::path found;
    for (const auto& e : std::filesystem::recursive_directory_iterator(work))
        if (e.path().extension() == ".mp4") { found = e.path(); break; }
    if (found.empty()) {
        std::filesystem::remove_all(work);
        throw std::runtime_error("ManimAdapter: output mp4 not found after render");
    }
    std::filesystem::create_directories(plan.output_path.parent_path());
    std::filesystem::copy_file(found, plan.output_path, std::filesystem::copy_options::overwrite_existing);
    std::filesystem::remove_all(work);

    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = plan.output_path;
    art.format = OutputFormat::Mp4;
    art.file_size_bytes = std::filesystem::file_size(plan.output_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - t0);
    art.stdout_text = result.stdout_text;
    art.stderr_text = result.stderr_text;
    return art;
}
HealthStatus ManimAdapter::health_check() const {
    bool ok = binary_exists("manim");
    return {ok, {}, ok ? "manim found on PATH" : "manim not found — install via `pip install manim`"};
}

// ─── RemotionAdapter — full implementation ───────────────────────────────────

std::string RemotionAdapter::name()    const { return "remotion"; }
std::string RemotionAdapter::version() const { return "0.1.0"; }
std::vector<NodeType> RemotionAdapter::supported_node_types() const {
    return {NodeType::Shape, NodeType::Text, NodeType::Image};
}
RenderPlan RemotionAdapter::compile(const Scene& scene, const RenderSettings& settings,
                                    const std::filesystem::path& output_dir) const {
    RenderPlan plan;
    plan.provider    = name();
    plan.output_format = OutputFormat::Mp4;
    plan.output_path = output_dir / (scene.id.value + ".mp4");
    // Store the generated TSX so callers can inspect the compilation output
    plan.source_text = scene_to_remotion_tsx(scene, settings);
    return plan;
}
RenderArtifact RemotionAdapter::execute(const RenderPlan& plan) const {
    const auto t0 = std::chrono::steady_clock::now();
    if (!plan.source_text) throw std::runtime_error("RemotionAdapter: missing source_text");

    // Semi-persistent workspace: node_modules is cached across renders
    auto work = std::filesystem::temp_directory_path() / "openanim-remotion-workspace";
    std::filesystem::create_directories(work / "src");

    bool needs_install = !std::filesystem::exists(work / "node_modules");

    // Always (re)write generated scene + boilerplate files
    { std::ofstream f(work/"src"/"GeneratedScene.tsx"); f << *plan.source_text; }

    if (needs_install) {
        // Write boilerplate project files on first use
        { std::ofstream f(work/"package.json");
          f << "{\n  \"name\": \"openanim-remotion\",\n  \"version\": \"0.1.0\",\n"
            << "  \"dependencies\": {\n"
            << "    \"react\": \"^18.2.0\",\n"
            << "    \"react-dom\": \"^18.2.0\",\n"
            << "    \"remotion\": \"^4.0.0\",\n"
            << "    \"@remotion/cli\": \"^4.0.0\"\n"
            << "  }\n}\n"; }
        { std::ofstream f(work/"tsconfig.json");
          f << R"({"compilerOptions":{"target":"ESNext","module":"commonjs","jsx":"react","strict":true}})" << '\n'; }
        { std::ofstream f(work/"src"/"index.tsx");
          f << "import {registerRoot} from 'remotion';\nimport {RemotionRoot} from './Root';\nregisterRoot(RemotionRoot);\n"; }

        auto inst = run_command("npm", {"install", "--prefer-offline"}, work, sandbox);
        if (inst.exit_code != 0)
            throw std::runtime_error("npm install failed: " + inst.stderr_text);
    }

    // Determine duration frames from file content (rough parse) — default 90
    uint32_t dur_frames = 90;
    { std::ofstream f(work/"src"/"Root.tsx");
      f << "import React from 'react';\n";
      f << "import {Composition} from 'remotion';\n";
      f << "import {GeneratedScene} from './GeneratedScene';\n\n";
      f << "export const RemotionRoot: React.FC = () => (\n";
      f << "  <Composition id=\"GeneratedScene\" component={GeneratedScene}\n";
      f << "    durationInFrames={" << dur_frames << "} fps={30}\n";
      f << "    width={1920} height={1080} />\n)\n"; }

    std::filesystem::create_directories(plan.output_path.parent_path());
    auto result = run_command("npx", {
        "remotion", "render",
        "src/index.tsx",
        "GeneratedScene",
        plan.output_path.string()
    }, work, sandbox);

    if (result.exit_code != 0)
        throw std::runtime_error("Remotion render failed (exit " + std::to_string(result.exit_code) + "):\n" + result.stderr_text);

    RenderArtifact art;
    art.plan_id = plan.id;
    art.output_path = plan.output_path;
    art.format = OutputFormat::Mp4;
    art.file_size_bytes = std::filesystem::file_size(plan.output_path);
    art.render_duration = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - t0);
    art.stdout_text = result.stdout_text;
    art.stderr_text = result.stderr_text;
    return art;
}
HealthStatus RemotionAdapter::health_check() const {
    if (!binary_exists("npx")) return {false, {}, "npx not found — install Node.js"};
    if (!binary_exists("node")) return {false, {}, "node not found — install Node.js"};
    return {true, {}, "npx+node found; remotion will be installed on first render"};
}

}  // namespace openanim
