#pragma once

#include "openanim/scene_ir.hpp"

#include <string>
#include <vector>

namespace openanim {

enum class ValidationCode {
    OrphanedParentRef,
    MissingChild,
    InconsistentParentChild,
    CycleDetected,
    MissingRootNode,
    RootHasParent,
    EventTargetMissing,
    TrackTargetMissing,
    DuplicateNodeId,
    EmptyScene
};

struct ValidationError {
    ValidationCode code;
    std::string message;
};

std::vector<ValidationError> validate_scene(const Scene& scene);
std::vector<ValidationError> validate_project(const Project& project);

}  // namespace openanim
