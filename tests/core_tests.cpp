#include "openanim/engine.hpp"
#include "openanim/hasher.hpp"

#include <cassert>
#include <filesystem>
#include <iostream>

int main() {
    auto scene = openanim::Scene::make("test");
    auto child = openanim::Node::named("circle", openanim::NodeType::Shape);
    child.parent = scene.root_node;
    child.components.transform = openanim::Transform::at(100.0, 100.0);
    child.components.style = openanim::Style::with_fill(openanim::Color{0.2, 0.6, 0.8, 1.0});
    openanim::Shape shape;
    shape.kind.type = openanim::ShapeType::Circle;
    shape.kind.radius = 50.0;
    child.components.shape = shape;
    scene.nodes.front().children.push_back(child.id);
    scene.nodes.push_back(child);

    auto errors = openanim::validate_scene(scene);
    assert(errors.empty());

    auto hash_a = openanim::hash_scene(scene);
    auto hash_b = openanim::hash_scene(scene);
    assert(hash_a == hash_b);
    assert(hash_a.size() == 64);

    auto project = openanim::Project::make("test");
    project.add_scene(scene);
    openanim::OpenAnimEngine engine(std::filesystem::temp_directory_path() / "openanim-cpp-test-cache");
    auto artifacts = engine.render_project(project);
    assert(artifacts.size() == 1);
    assert(std::filesystem::exists(artifacts.front().output_path));

    std::cout << "openanim-core tests passed\n";
    return 0;
}
