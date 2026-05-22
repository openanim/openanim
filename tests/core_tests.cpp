#include "openanim/engine.hpp"
#include "openanim/hasher.hpp"
#include "openanim/llm.hpp"
#include "openanim/renderer.hpp"

#include <cassert>
#include <filesystem>
#include <iomanip>
#include <iostream>
#include <sstream>

// ── Helpers ──────────────────────────────────────────────────────────────────

static openanim::Scene make_circle_scene(const std::string& name,
                                         openanim::Color fill,
                                         double cx, double cy,
                                         double radius) {
    auto scene = openanim::Scene::make(name);
    auto node  = openanim::Node::named("circle", openanim::NodeType::Shape);
    node.parent = scene.root_node;
    node.components.transform = openanim::Transform::at(cx, cy);
    node.components.style     = openanim::Style::with_fill(fill);
    openanim::Shape sh;
    sh.kind.type   = openanim::ShapeType::Circle;
    sh.kind.radius = radius;
    node.components.shape = sh;
    scene.nodes.front().children.push_back(node.id);
    scene.nodes.push_back(node);
    scene.duration = openanim::DurationSecs{3.0};
    return scene;
}

static openanim::Scene make_diagram_scene(openanim::DiagramLanguage lang,
                                           const std::string& source) {
    auto scene = openanim::Scene::make("diagram");
    auto node  = openanim::Node::named("diag", openanim::NodeType::Diagram);
    node.parent = scene.root_node;
    openanim::Diagram d;
    d.language = lang;
    d.source   = source;
    node.components.diagram = d;
    scene.nodes.front().children.push_back(node.id);
    scene.nodes.push_back(node);
    return scene;
}

static void print_health(const std::vector<std::pair<std::string, openanim::HealthStatus>>& health) {
    std::cout << "\n── Renderer Health ──────────────────────────────────────\n";
    for (const auto& [name, status] : health) {
        std::cout << "  [" << (status.available ? "✓" : "✗") << "] "
                  << std::left << std::setw(12) << name;
        if (status.message) std::cout << "  " << *status.message;
        std::cout << "\n";
    }
    std::cout << "─────────────────────────────────────────────────────────\n\n";
}

int main() {
    std::cout << "=== openanim-core tests ===\n\n";

    auto cache_dir = std::filesystem::temp_directory_path() / "openanim-core-test-cache";
    openanim::OpenAnimEngine engine(cache_dir);

    // ── Test 1: Scene construction ─────────────────────────────────────────
    std::cout << "[1] Scene construction... ";
    auto scene = make_circle_scene("test", {0.2, 0.6, 0.8, 1.0}, 960, 540, 100);
    std::cout << "OK\n";

    // ── Test 2: Validation ─────────────────────────────────────────────────
    std::cout << "[2] Validation... ";
    auto errors = openanim::validate_scene(scene);
    assert(errors.empty() && "scene should have no validation errors");
    std::cout << "OK\n";

    // ── Test 3: Hash determinism ────────────────────────────────────────────
    std::cout << "[3] Hash determinism... ";
    auto hash_a = openanim::hash_scene(scene);
    auto hash_b = openanim::hash_scene(scene);
    assert(hash_a == hash_b && "same scene must produce same hash");
    assert(hash_a.size() == 64 && "hash must be 64 hex chars");
    std::cout << "OK  hash=" << hash_a.substr(0, 16) << "...\n";

    // ── Test 4: SVG render ─────────────────────────────────────────────────
    std::cout << "[4] SVG render... ";
    auto project = openanim::Project::make("test");
    project.settings.format = openanim::OutputFormat::Svg;
    project.add_scene(scene);
    auto artifacts = engine.render_project(project);
    assert(artifacts.size() == 1);
    assert(std::filesystem::exists(artifacts.front().output_path));
    std::cout << "OK  " << artifacts.front().output_path << "\n";

    // ── Test 5: Cache hit ──────────────────────────────────────────────────
    std::cout << "[5] Cache hit... ";
    auto artifacts2 = engine.render_project(project);
    assert(artifacts2.front().status == openanim::ArtifactStatus::Cached);
    std::cout << "OK\n";

    // ── Test 6: JSON round-trip ────────────────────────────────────────────
    std::cout << "[6] JSON round-trip... ";
    auto json_val  = openanim::to_json(project);
    auto recovered = openanim::project_from_json(json_val);
    assert(recovered.metadata.name == project.metadata.name);
    assert(recovered.scenes.size() == project.scenes.size());
    std::cout << "OK\n";

    // ── Test 7: Renderer health check ─────────────────────────────────────
    std::cout << "[7] Renderer health check... ";
    auto health = engine.health_check_all();
    assert(!health.empty());
    bool svg_ok = false;
    for (const auto& [name, status] : health)
        if (name == "svg" && status.available) svg_ok = true;
    assert(svg_ok && "SVG adapter must always be available");
    std::cout << "OK\n";

    // ── Test 8: Manim Python code generation ──────────────────────────────
    std::cout << "[8] Manim Python IR generation... ";
    {
        // Build a scene with circle + text + math + position animation
        auto manim_scene = openanim::Scene::make("manim-test");
        manim_scene.duration = openanim::DurationSecs{3.0};

        // Shape node
        auto circ = openanim::Node::named("c1", openanim::NodeType::Shape);
        circ.parent = manim_scene.root_node;
        circ.components.transform = openanim::Transform::at(200.0, 540.0);
        circ.components.style = openanim::Style::with_fill({0.2, 0.4, 1.0, 1.0});
        openanim::Shape sh; sh.kind.type = openanim::ShapeType::Circle; sh.kind.radius = 80.0;
        circ.components.shape = sh;
        manim_scene.nodes.front().children.push_back(circ.id);
        manim_scene.nodes.push_back(circ);

        // Text node
        auto txt = openanim::Node::named("t1", openanim::NodeType::Text);
        txt.parent = manim_scene.root_node;
        txt.components.transform = openanim::Transform::at(960.0, 200.0);
        openanim::TextContent text_comp;
        text_comp.content = "OpenAnim Engine";
        text_comp.font.size = 48.0;
        txt.components.text = text_comp;
        manim_scene.nodes.front().children.push_back(txt.id);
        manim_scene.nodes.push_back(txt);

        // Math node
        auto math = openanim::Node::named("m1", openanim::NodeType::Math);
        math.parent = manim_scene.root_node;
        math.components.transform = openanim::Transform::at(960.0, 800.0);
        openanim::MathExpression math_comp;
        math_comp.latex = "E = mc^2";
        math_comp.font_size = 48.0;
        math_comp.color = {1.0, 1.0, 1.0, 1.0};
        math.components.math = math_comp;
        manim_scene.nodes.front().children.push_back(math.id);
        manim_scene.nodes.push_back(math);

        // Add a position-x animation track for the circle
        openanim::KeyframeTrack track;
        track.target_node = circ.id;
        track.property = openanim::AnimatableProperty::PositionX;
        track.keyframes = {
            {openanim::DurationSecs{0.0}, 200.0,  openanim::EasingFunction::Linear},
            {openanim::DurationSecs{3.0}, 1720.0, openanim::EasingFunction::Linear}
        };
        manim_scene.timeline.tracks.push_back(track);

        openanim::RenderSettings settings;
        openanim::ManimAdapter adapter;
        auto plan = adapter.compile(manim_scene, settings, cache_dir);

        assert(plan.source_text.has_value());
        const auto& py = *plan.source_text;

        // Structural checks on generated Python
        assert(py.find("from manim import *") != std::string::npos);
        assert(py.find("class GeneratedScene(Scene)") != std::string::npos);
        assert(py.find("Circle(") != std::string::npos);
        assert(py.find("Text(") != std::string::npos);
        assert(py.find("MathTex(") != std::string::npos);
        assert(py.find("move_to") != std::string::npos || py.find("set_x") != std::string::npos);
        assert(py.find("E = mc^2") != std::string::npos);
        assert(py.find("OpenAnim Engine") != std::string::npos);
        std::cout << "OK  (" << py.size() << " bytes Python)\n";
    }

    // ── Test 9: Remotion TSX code generation ──────────────────────────────
    std::cout << "[9] Remotion TSX IR generation... ";
    {
        auto rem_scene = openanim::Scene::make("remotion-test");
        rem_scene.duration = openanim::DurationSecs{3.0};
        auto rect = openanim::Node::named("r1", openanim::NodeType::Shape);
        rect.parent = rem_scene.root_node;
        rect.components.transform = openanim::Transform::at(100.0, 540.0);
        rect.components.style = openanim::Style::with_fill({1.0, 0.3, 0.3, 1.0});
        openanim::Shape sh; sh.kind.type = openanim::ShapeType::Rectangle;
        sh.kind.width = 200.0; sh.kind.height = 200.0;
        rect.components.shape = sh;
        rem_scene.nodes.front().children.push_back(rect.id);
        rem_scene.nodes.push_back(rect);

        openanim::KeyframeTrack track;
        track.target_node = rect.id;
        track.property = openanim::AnimatableProperty::PositionX;
        track.keyframes = {
            {openanim::DurationSecs{0.0}, 100.0,  openanim::EasingFunction::Linear},
            {openanim::DurationSecs{3.0}, 1820.0, openanim::EasingFunction::Linear}
        };
        rem_scene.timeline.tracks.push_back(track);

        openanim::RenderSettings settings;
        openanim::RemotionAdapter adapter;
        auto plan = adapter.compile(rem_scene, settings, cache_dir);

        assert(plan.source_text.has_value());
        const auto& tsx = *plan.source_text;

        assert(tsx.find("import React") != std::string::npos);
        assert(tsx.find("useCurrentFrame") != std::string::npos);
        assert(tsx.find("interpolate") != std::string::npos);
        assert(tsx.find("AbsoluteFill") != std::string::npos);
        assert(tsx.find("borderRadius") != std::string::npos); // rectangle uses css border-radius
        std::cout << "OK  (" << tsx.size() << " bytes TSX)\n";
    }

    // ── Test 10: Multi-scene project + SVG render ─────────────────────────
    std::cout << "[10] Multi-scene project render... ";
    {
        auto proj2 = openanim::Project::make("multi");
        proj2.settings.format = openanim::OutputFormat::Svg;
        proj2.add_scene(make_circle_scene("s1", {1.0, 0.2, 0.2, 1.0}, 480,  540, 80));
        proj2.add_scene(make_circle_scene("s2", {0.2, 1.0, 0.2, 1.0}, 1440, 540, 80));
        auto arts = engine.render_project(proj2);
        assert(arts.size() == 2);
        assert(std::filesystem::exists(arts[0].output_path));
        assert(std::filesystem::exists(arts[1].output_path));
        // Hashes must differ between scenes
        assert(arts[0].output_path != arts[1].output_path);
        std::cout << "OK  2 scenes rendered\n";
    }

    // ── Test 11: PlantUML server render ───────────────────────────────────
    std::cout << "[11] PlantUML server render... ";
    {
        openanim::PlantUmlAdapter plantuml;
        auto health = plantuml.health_check();
        if (!health.available) {
            std::cout << "SKIP (server unreachable)\n";
        } else {
            auto diag_scene = make_diagram_scene(
                openanim::DiagramLanguage::PlantUml,
                "@startuml\nAlice -> Bob: hello\nBob --> Alice: hi!\n@enduml");
            openanim::RenderSettings settings;
            auto plan = plantuml.compile(diag_scene, settings, cache_dir);
            auto art  = plantuml.execute(plan);
            assert(std::filesystem::exists(art.output_path));
            assert(art.file_size_bytes > 100);
            std::cout << "OK  " << art.output_path << " (" << art.file_size_bytes << " bytes)\n";
        }
    }

    // ── Test 12: FFmpeg MP4 render (single scene) ─────────────────────────
    std::cout << "[12] FFmpeg MP4 render... ";
    {
        openanim::FfmpegAdapter ffmpeg;
        auto health = ffmpeg.health_check();
        if (!health.available) {
            std::cout << "SKIP (ffmpeg not found)\n";
        } else {
            auto mp4_scene = make_circle_scene("mp4test", {0.8, 0.5, 0.1, 1.0}, 960, 540, 150);
            openanim::RenderSettings settings;
            settings.format = openanim::OutputFormat::Mp4;
            auto plan = ffmpeg.compile(mp4_scene, settings, cache_dir);
            auto art  = ffmpeg.execute(plan);
            assert(std::filesystem::exists(art.output_path));
            assert(art.file_size_bytes > 0);
            std::cout << "OK  " << art.output_path << " (" << art.file_size_bytes << " bytes)\n";
        }
    }

    print_health(health);
    std::cout << "All core tests passed ✓\n";
    return 0;
}
