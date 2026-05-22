#pragma once

#include "openanim/llm.hpp"
#include "openanim/renderer.hpp"
#include "openanim/scene_ir.hpp"

#include <string>
#include <vector>

namespace openanim {

// A render task assigned by the planner to a specific provider
struct RenderTask {
    std::string scene_id;
    std::string provider_name;
    RenderOptions options;
};

// Abstract base for all agents
class Agent {
public:
    virtual ~Agent() = default;
    virtual std::string name() const = 0;
};

// Compiles a natural language prompt into a complete Project IR.
// Implemented in the SaaS layer using LlmCompiler internally.
class CompilerAgent : public Agent {
public:
    virtual Project compile(std::string_view prompt, const LlmProvider& provider) = 0;
};

// Given a failed project + error context, repairs the IR and returns a fixed project.
// Drives the self-repair loop described in Philosophy.txt (OpenHands pattern).
class RepairAgent : public Agent {
public:
    virtual Project repair(
        const Project& project,
        std::string_view error_context,
        const LlmProvider& provider) = 0;
};

// Plans which renderer adapter to use for each scene based on node types and constraints.
// Later versions may use an LLM for dynamic planning.
class PlannerAgent : public Agent {
public:
    virtual std::vector<RenderTask> plan(const Project& project) = 0;
};

// Selects the best available provider for a single scene.
// Examines node types and registered adapters to make a deterministic choice.
class ProviderSelectorAgent : public Agent {
public:
    virtual std::string select_provider(const Scene& scene) = 0;
};

}  // namespace openanim
