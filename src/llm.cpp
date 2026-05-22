#include "openanim/llm.hpp"
#include "openanim/validation.hpp"

#include <boost/json.hpp>
#include <curl/curl.h>

#include <cstdlib>
#include <sstream>
#include <stdexcept>

namespace openanim {
namespace {

// ─── System prompt ───────────────────────────────────────────────────────────
// Teaches the LLM the full project.json schema. Updated to match the C++ IR.

static const char* COMPILE_SYSTEM_PROMPT = R"(You are Antigravity-Coder, the compiler for the OpenAnim animation engine.
Translate natural language animation requests into a valid project.json file matching the OpenAnim Scene IR schema.
Output ONLY raw valid JSON — no markdown, no explanation, no code blocks.

TOP-LEVEL PROJECT SCHEMA:
{
  "id": "<uuid-v4>",
  "metadata": {"name": "<string>", "version": "0.1.0", "authors": []},
  "scenes": [ <scene>, ... ],
  "assets": {},
  "settings": {
    "resolution": [1920, 1080],
    "fps": 30.0,
    "format": "svg",
    "background_color": {"r": 0.05, "g": 0.05, "b": 0.05, "a": 1.0},
    "anti_aliasing": 4,
    "pixel_scale": 1.0
  }
}

SCENE:
{
  "id": "<uuid-v4>",
  "name": "<string>",
  "root_node": "<uuid matching root node id>",
  "nodes": [ <node>, ... ],
  "timeline": {"duration": <float>, "tracks": [ <track>, ... ], "events": []},
  "duration": <float>,
  "metadata": {}
}

NODE:
{
  "id": "<uuid-v4>",
  "name": "<string>",
  "node_type": "group" | "shape" | "text" | "math" | "diagram",
  "parent": "<parent-uuid>" | null,
  "children": ["<child-uuid>", ...],
  "components": { <component-fields> }
}

COMPONENT FIELDS (include only relevant ones):
- "transform": {"position": {"x": <f>, "y": <f>}, "rotation": <f>, "scale": {"x": 1.0, "y": 1.0}, "anchor": {"x": 0.5, "y": 0.5}}
- "style": {"fill": {"r": <f>, "g": <f>, "b": <f>, "a": 1.0}, "stroke": {"color": {"r":<f>,"g":<f>,"b":<f>,"a":1.0}, "width": <f>, "line_cap": "butt", "line_join": "miter"}, "opacity": 1.0, "z_index": 0, "visible": true}
- "shape": {"kind": {"type": "circle", "radius": <f>} | {"type": "rectangle", "width": <f>, "height": <f>, "corner_radius": <f>} | {"type": "line", "start": {"x":<f>,"y":<f>}, "end": {"x":<f>,"y":<f>}} | {"type": "ellipse", "rx": <f>, "ry": <f>}}
- "text": {"content": "<string>", "font": {"family": "Inter", "size": 24.0, "weight": "regular", "style": "normal"}, "align": "left"|"center"|"right", "line_height": 1.4}
- "math": {"latex": "<latex>", "font_size": 36.0, "color": {"r":1.0,"g":1.0,"b":1.0,"a":1.0}}
- "diagram": {"source": "<source-code>", "language": "mermaid" | "plant_uml"}

KEYFRAME TRACK:
{"target_node": "<uuid>", "property": "position"|"opacity"|"rotation"|"scale"|"position_x"|"position_y", "keyframes": [{"time": <f>, "value": {"type": "scalar", "value": <f>}, "easing": "linear"|"ease_in"|"ease_out"|"ease_in_out"}]}

CRITICAL RULES:
- All UUIDs must be unique v4 UUIDs (e.g. "a1b2c3d4-e5f6-7890-abcd-ef1234567890")
- "root_node" UUID must match exactly one node with "parent": null
- Every node's "parent" must reference an existing node id; every child id in "children" must exist
- No cycles in parent-child graph
- Color channels are floats 0.0–1.0, NOT 0–255
- "format" must be "svg" unless video output is explicitly requested
- Produce ONLY the JSON object, starting with { and ending with })";

static const char* PATCH_SYSTEM_PROMPT = R"(You are Antigravity-Coder, the patch compiler for the OpenAnim animation engine.
You will receive a current project.json and a modification request.
Output the complete, modified project.json with only the requested changes applied.
Keep all existing IDs stable where possible — only change what is necessary.
Output ONLY raw valid JSON — no markdown, no explanation, no code blocks.)";

// ─── libcurl HTTP client ──────────────────────────────────────────────────────

static size_t curl_write_cb(void* contents, size_t size, size_t nmemb, std::string* out) {
    out->append(static_cast<char*>(contents), size * nmemb);
    return size * nmemb;
}

static std::string http_post(
    const std::string& url,
    const std::string& body,
    const std::vector<std::string>& headers)
{
    CURL* curl = curl_easy_init();
    if (!curl) throw std::runtime_error("curl_easy_init() failed");

    std::string response;
    long http_code = 0;
    char errbuf[CURL_ERROR_SIZE] = {};

    struct curl_slist* header_list = nullptr;
    for (const auto& h : headers) header_list = curl_slist_append(header_list, h.c_str());

    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());
    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, header_list);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, curl_write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
    curl_easy_setopt(curl, CURLOPT_ERRORBUFFER, errbuf);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 120L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);

    CURLcode res = curl_easy_perform(curl);
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);
    curl_slist_free_all(header_list);
    curl_easy_cleanup(curl);

    if (res != CURLE_OK) {
        throw std::runtime_error(std::string("curl error: ") + errbuf);
    }
    if (http_code >= 400) {
        throw std::runtime_error("HTTP " + std::to_string(http_code) + ": " + response);
    }
    return response;
}

static std::string http_get(const std::string& url) {
    CURL* curl = curl_easy_init();
    if (!curl) throw std::runtime_error("curl_easy_init() failed");

    std::string response;
    long http_code = 0;
    char errbuf[CURL_ERROR_SIZE] = {};

    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, curl_write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
    curl_easy_setopt(curl, CURLOPT_ERRORBUFFER, errbuf);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);

    CURLcode res = curl_easy_perform(curl);
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &http_code);
    curl_easy_cleanup(curl);

    if (res != CURLE_OK) throw std::runtime_error(std::string("curl error: ") + errbuf);
    if (http_code >= 400) throw std::runtime_error("HTTP GET " + std::to_string(http_code) + ": " + url);
    return response;
}

// ─── Provider routing ─────────────────────────────────────────────────────────

static std::string query_openai_compat(
    const std::string& url,
    const std::string& api_key,
    const std::string& model,
    std::string_view system_prompt,
    std::string_view user_prompt)
{
    namespace json = boost::json;

    json::object body;
    body["model"] = model;
    json::array messages;
    messages.push_back(json::object{{"role", "system"}, {"content", std::string(system_prompt)}});
    messages.push_back(json::object{{"role", "user"}, {"content", std::string(user_prompt)}});
    body["messages"] = messages;
    body["temperature"] = 0.1;

    std::vector<std::string> headers = {
        "Content-Type: application/json",
        "Authorization: Bearer " + api_key,
    };

    auto resp = http_post(url, json::serialize(body), headers);
    auto parsed = json::parse(resp);
    return std::string(parsed.as_object()
        .at("choices").as_array()
        .at(0).as_object()
        .at("message").as_object()
        .at("content").as_string());
}

static std::string query_anthropic(
    const std::string& api_key,
    const std::string& model,
    std::string_view system_prompt,
    std::string_view user_prompt)
{
    namespace json = boost::json;

    json::object body;
    body["model"] = model;
    body["max_tokens"] = 8192;
    body["system"] = std::string(system_prompt);
    json::array messages;
    messages.push_back(json::object{{"role", "user"}, {"content", std::string(user_prompt)}});
    body["messages"] = messages;
    body["temperature"] = 0.1;

    std::vector<std::string> headers = {
        "Content-Type: application/json",
        "x-api-key: " + api_key,
        "anthropic-version: 2023-06-01",
    };

    auto resp = http_post("https://api.anthropic.com/v1/messages", json::serialize(body), headers);
    auto parsed = json::parse(resp);
    return std::string(parsed.as_object()
        .at("content").as_array()
        .at(0).as_object()
        .at("text").as_string());
}

static std::string query_ollama(
    const std::string& base_url,
    const std::string& model,
    std::string_view system_prompt,
    std::string_view user_prompt)
{
    namespace json = boost::json;

    json::object body;
    body["model"] = model;
    body["stream"] = false;
    json::array messages;
    messages.push_back(json::object{{"role", "system"}, {"content", std::string(system_prompt)}});
    messages.push_back(json::object{{"role", "user"}, {"content", std::string(user_prompt)}});
    body["messages"] = messages;
    json::object options;
    options["temperature"] = 0.1;
    body["options"] = options;

    std::string url = base_url;
    if (!url.empty() && url.back() == '/') url.pop_back();
    url += "/api/chat";

    std::vector<std::string> headers = {"Content-Type: application/json"};
    auto resp = http_post(url, json::serialize(body), headers);
    auto parsed = json::parse(resp);
    return std::string(parsed.as_object()
        .at("message").as_object()
        .at("content").as_string());
}

// ─── Repair prompt builder ────────────────────────────────────────────────────

static std::string build_repair_prompt(const std::string& bad_json, const std::string& error) {
    return "Your previous output failed validation.\n"
           "Error: " + error + "\n\n"
           "Bad JSON:\n" + bad_json + "\n\n"
           "Fix the error and output the corrected project.json. Output ONLY raw JSON.";
}

static std::string build_patch_user_prompt(
    const std::string& project_json,
    std::string_view modification)
{
    return "Current project:\n" + project_json +
           "\n\nModification requested: " + std::string(modification) +
           "\n\nOutput the complete modified project.json. Output ONLY raw JSON.";
}

}  // namespace

// ─── LlmProvider factories ────────────────────────────────────────────────────

LlmProvider LlmProvider::openai(std::string api_key, std::string model) {
    return {LlmProviderType::OpenAI, "https://api.openai.com/v1", std::move(api_key), std::move(model)};
}

LlmProvider LlmProvider::anthropic(std::string api_key, std::string model) {
    return {LlmProviderType::Anthropic, "https://api.anthropic.com", std::move(api_key), std::move(model)};
}

LlmProvider LlmProvider::ollama(std::string base_url, std::string model) {
    return {LlmProviderType::Ollama, std::move(base_url), {}, std::move(model)};
}

LlmProvider LlmProvider::openrouter(std::string api_key, std::string model) {
    return {LlmProviderType::OpenAI, "https://openrouter.ai/api/v1", std::move(api_key), std::move(model)};
}

LlmProvider LlmProvider::from_env() {
    LlmProvider p;
    if (const char* v = std::getenv("OPENANIM_LLM_BASE_URL")) p.base_url = v;
    if (const char* v = std::getenv("OPENANIM_LLM_API_KEY"))  p.api_key  = v;
    if (const char* v = std::getenv("OPENANIM_LLM_MODEL"))    p.model    = v;

    // Infer type from base URL
    if (p.base_url.find("anthropic") != std::string::npos) {
        p.type = LlmProviderType::Anthropic;
    } else if (p.base_url.find("11434") != std::string::npos ||
               p.base_url.find("ollama") != std::string::npos) {
        p.type = LlmProviderType::Ollama;
    } else {
        p.type = LlmProviderType::OpenAI; // covers OpenAI, OpenRouter, and any OpenAI-compatible API
    }
    return p;
}

// ─── LlmCompiler ─────────────────────────────────────────────────────────────

LlmCompiler::LlmCompiler(LlmProvider provider) : provider_(std::move(provider)) {}

std::string LlmCompiler::query_llm(std::string_view system_prompt, std::string_view user_prompt) {
    switch (provider_.type) {
        case LlmProviderType::Anthropic:
            return query_anthropic(provider_.api_key, provider_.model, system_prompt, user_prompt);

        case LlmProviderType::Ollama:
            return query_ollama(provider_.base_url, provider_.model, system_prompt, user_prompt);

        case LlmProviderType::OpenAI:
        case LlmProviderType::OpenRouter: {
            // Both use the OpenAI wire format; base_url points to the right server
            std::string url = provider_.base_url;
            if (!url.empty() && url.back() == '/') url.pop_back();
            url += "/chat/completions";
            return query_openai_compat(url, provider_.api_key, provider_.model, system_prompt, user_prompt);
        }
    }
    throw std::runtime_error("Unknown LLM provider type");
}

std::string LlmCompiler::clean_json_response(std::string_view raw) {
    std::string s(raw);
    // Trim leading whitespace
    auto start = s.find_first_not_of(" \t\n\r");
    if (start == std::string::npos) return s;
    s = s.substr(start);

    // Strip markdown code fence: ```json...``` or ```...```
    if (s.rfind("```", 0) == 0) {
        auto newline = s.find('\n');
        if (newline != std::string::npos) s = s.substr(newline + 1);
        if (s.size() >= 3 && s.substr(s.size() - 3) == "```") s = s.substr(0, s.size() - 3);
    }

    // Trim again
    start = s.find_first_not_of(" \t\n\r");
    auto end = s.find_last_not_of(" \t\n\r");
    if (start == std::string::npos) return s;
    return s.substr(start, end - start + 1);
}

Project LlmCompiler::compile_with_retry(std::string_view initial_user_prompt, int max_attempts) {
    std::string user_prompt(initial_user_prompt);
    std::string last_error;
    std::string bad_json;
    bool is_repair = false;

    for (int attempt = 0; attempt < max_attempts; ++attempt) {
        std::string raw;
        try {
            const char* sys = is_repair ? PATCH_SYSTEM_PROMPT : COMPILE_SYSTEM_PROMPT;
            raw = query_llm(sys, user_prompt);
        } catch (const std::exception& e) {
            throw std::runtime_error(std::string("LLM query failed: ") + e.what());
        }

        auto cleaned = clean_json_response(raw);

        try {
            auto parsed = boost::json::parse(cleaned);
            auto project = project_from_json(parsed);

            // Validate referential integrity
            auto errors = validate_project(project);
            if (errors.empty()) return project;

            std::ostringstream err_msg;
            for (const auto& e : errors) err_msg << "  - " << e.message << "\n";
            bad_json = cleaned;
            last_error = "Validation errors:\n" + err_msg.str();
        } catch (const std::exception& e) {
            bad_json = cleaned;
            last_error = std::string("JSON parse error: ") + e.what();
        }

        // Prepare repair prompt for next attempt
        user_prompt = build_repair_prompt(bad_json, last_error);
        is_repair = true;
    }

    throw std::runtime_error(
        "LLM compiler failed after " + std::to_string(max_attempts) + " attempts.\n"
        "Last error: " + last_error + "\nLast JSON:\n" + bad_json);
}

Project LlmCompiler::compile_project(std::string_view prompt) {
    return compile_with_retry(prompt, 3);
}

Project LlmCompiler::patch_project(const Project& current, std::string_view patch_prompt) {
    auto current_json = boost::json::serialize(to_json(current));
    auto user_prompt = build_patch_user_prompt(current_json, patch_prompt);
    return compile_with_retry(user_prompt, 3);
}

// Expose a standalone HTTP GET for PlantUML server (used by PlantUmlAdapter)
std::string llm_http_get(const std::string& url) {
    return http_get(url);
}

}  // namespace openanim
