#include "openanim/validation.hpp"

#include <functional>
#include <unordered_map>
#include <unordered_set>

namespace openanim {
namespace {

struct IdHash {
    std::size_t operator()(const Id& id) const noexcept {
        return std::hash<std::string>{}(id.value);
    }
};

void add(std::vector<ValidationError>& errors, ValidationCode code, std::string message) {
    errors.push_back({code, std::move(message)});
}

}  // namespace

std::vector<ValidationError> validate_scene(const Scene& scene) {
    std::vector<ValidationError> errors;
    if (scene.nodes.empty()) {
        add(errors, ValidationCode::EmptyScene, "Scene has no nodes");
        return errors;
    }

    std::unordered_map<Id, const Node*, IdHash> nodes;
    std::unordered_set<Id, IdHash> seen;
    for (const auto& node : scene.nodes) {
        if (!seen.insert(node.id).second) {
            add(errors, ValidationCode::DuplicateNodeId, "Duplicate node ID " + node.id.value + " in scene");
        }
        nodes[node.id] = &node;
    }

    auto root_it = nodes.find(scene.root_node);
    if (root_it == nodes.end()) {
        add(errors, ValidationCode::MissingRootNode, "Root node " + scene.root_node.value + " not found in scene nodes");
    } else if (root_it->second->parent) {
        add(errors, ValidationCode::RootHasParent, "Root node " + scene.root_node.value + " has a parent set");
    }

    for (const auto& node : scene.nodes) {
        if (node.parent && !nodes.contains(*node.parent)) {
            add(errors, ValidationCode::OrphanedParentRef, "Node " + node.id.value + " references non-existent parent " + node.parent->value);
        }

        for (const auto& child_id : node.children) {
            auto child_it = nodes.find(child_id);
            if (child_it == nodes.end()) {
                add(errors, ValidationCode::MissingChild, "Node " + node.id.value + " lists child " + child_id.value + " which does not exist");
                continue;
            }
            if (!child_it->second->parent || *child_it->second->parent != node.id) {
                add(errors, ValidationCode::InconsistentParentChild, "Node " + child_id.value + " is listed as child of " + node.id.value + " but has a different parent");
            }
        }
    }

    std::unordered_set<Id, IdHash> visited;
    std::unordered_set<Id, IdHash> stack;
    std::function<bool(const Id&)> dfs = [&](const Id& id) {
        visited.insert(id);
        stack.insert(id);
        auto it = nodes.find(id);
        if (it != nodes.end()) {
            for (const auto& child : it->second->children) {
                if (!visited.contains(child)) {
                    if (dfs(child)) return true;
                } else if (stack.contains(child)) {
                    add(errors, ValidationCode::CycleDetected, "Cycle detected in node hierarchy involving node " + child.value);
                    return true;
                }
            }
        }
        stack.erase(id);
        return false;
    };

    for (const auto& node : scene.nodes) {
        if (!visited.contains(node.id)) {
            dfs(node.id);
        }
    }

    for (const auto& track : scene.timeline.tracks) {
        if (!nodes.contains(track.target_node)) {
            add(errors, ValidationCode::TrackTargetMissing, "Timeline track references non-existent node " + track.target_node.value);
        }
    }

    for (const auto& event : scene.timeline.events) {
        for (const auto& target : event.target_nodes) {
            if (!nodes.contains(target)) {
                add(errors, ValidationCode::EventTargetMissing, "Timeline event references non-existent node " + target.value);
            }
        }
    }

    return errors;
}

std::vector<ValidationError> validate_project(const Project& project) {
    std::vector<ValidationError> errors;
    for (const auto& scene : project.scenes) {
        auto scene_errors = validate_scene(scene);
        errors.insert(errors.end(), scene_errors.begin(), scene_errors.end());
    }
    return errors;
}

}  // namespace openanim
