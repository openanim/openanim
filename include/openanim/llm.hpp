#pragma once

#include "openanim/scene_ir.hpp"
#include <string>
#include <string_view>

namespace openanim {

// Which LLM API to talk to
enum class LlmProviderType { OpenAI, Anthropic, Ollama, OpenRouter };

// Provider configuration — all fields needed to make an authenticated API call
struct LlmProvider {
    LlmProviderType type = LlmProviderType::OpenAI;
    std::string base_url;
    std::string api_key;
    std::string model;

    // Factory helpers
    static LlmProvider openai(std::string api_key, std::string model = "gpt-4o-mini");
    static LlmProvider anthropic(std::string api_key, std::string model = "claude-3-5-sonnet-20241022");
    static LlmProvider ollama(std::string base_url = "http://localhost:11434", std::string model = "llama3");
    // OpenRouter is OpenAI-compatible — same wire format, different base URL
    static LlmProvider openrouter(std::string api_key, std::string model = "openai/gpt-4o-mini");
    // Read OPENANIM_LLM_BASE_URL / OPENANIM_LLM_API_KEY / OPENANIM_LLM_MODEL from env
    static LlmProvider from_env();
};

// Compiles natural language into Scene IR and patches existing projects
class LlmCompiler {
public:
    explicit LlmCompiler(LlmProvider provider);

    // Compile a prompt into a complete, validated Project IR
    Project compile_project(std::string_view prompt);

    // Apply a modification prompt to an existing project, returning a new project
    // with only the necessary changes made (stable IDs where possible)
    Project patch_project(const Project& current, std::string_view patch_prompt);

private:
    // Low-level: send system+user prompt to the configured LLM and return raw text
    std::string query_llm(std::string_view system_prompt, std::string_view user_prompt);

    // Strip markdown code fences and leading/trailing whitespace from an LLM response
    static std::string clean_json_response(std::string_view raw);

    // Compile/repair loop — tries up to max_attempts times, sending repair prompts on failure
    Project compile_with_retry(std::string_view user_prompt, int max_attempts = 3);

    LlmProvider provider_;
};

}  // namespace openanim
