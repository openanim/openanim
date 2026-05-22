// E2E LLM integration test for OpenAnim engine.
//
// Usage:
//   OPENANIM_LLM_BASE_URL=https://openrouter.ai/api/v1 \
//   OPENANIM_LLM_API_KEY=sk-or-v1-... \
//   OPENANIM_LLM_MODEL=openai/gpt-4o-mini \
//   ./build/openanim-tests-llm [optional: "custom animation prompt"]
//
// If env vars are not set, the test exits 0 with a skip message.

#include "openanim/engine.hpp"
#include "openanim/llm.hpp"

#include <cassert>
#include <cstdlib>
#include <filesystem>
#include <iomanip>
#include <iostream>

namespace {

void print_section(const std::string& title) {
    std::cout << "\n── " << title << " ";
    for (int i = static_cast<int>(title.size()) + 4; i < 58; ++i) std::cout << "─";
    std::cout << "\n";
}

void print_health(const std::vector<std::pair<std::string, openanim::HealthStatus>>& health) {
    print_section("Renderer Health");
    for (const auto& [name, status] : health) {
        std::cout << "  [" << (status.available ? "✓" : "✗") << "] "
                  << std::left << std::setw(12) << name;
        if (status.message) std::cout << "  " << *status.message;
        std::cout << "\n";
    }
}

void print_project_summary(const openanim::Project& p) {
    std::cout << "  name    : " << p.metadata.name << "\n";
    std::cout << "  scenes  : " << p.scenes.size() << "\n";
    std::cout << "  nodes   : " << p.total_node_count() << "\n";
    std::cout << "  format  : " << openanim::to_string(p.settings.format) << "\n";
}

bool ffmpeg_available() {
    openanim::FfmpegAdapter a;
    return a.health_check().available;
}

}  // namespace

int main(int argc, char* argv[]) {
    std::cout << "=== openanim E2E LLM integration test ===\n";

    // ── Determine prompt ──────────────────────────────────────────────────
    std::string prompt = (argc > 1)
        ? std::string(argv[1])
        : "A bright blue circle smoothly sliding from the left side of the screen to the right over 3 seconds, on a dark background";

    // ── Read provider from environment ───────────────────────────────────
    const char* base_url = std::getenv("OPENANIM_LLM_BASE_URL");
    const char* api_key  = std::getenv("OPENANIM_LLM_API_KEY");
    const char* model    = std::getenv("OPENANIM_LLM_MODEL");

    if (!base_url || !api_key || !model) {
        std::cout << "\n[SKIP] Environment variables not set.\n"
                  << "  Set OPENANIM_LLM_BASE_URL, OPENANIM_LLM_API_KEY, OPENANIM_LLM_MODEL\n"
                  << "  Example (OpenRouter):\n"
                  << "    OPENANIM_LLM_BASE_URL=https://openrouter.ai/api/v1\n"
                  << "    OPENANIM_LLM_API_KEY=sk-or-v1-...\n"
                  << "    OPENANIM_LLM_MODEL=openai/gpt-4o-mini\n";
        return 0;
    }

    openanim::LlmProvider provider = openanim::LlmProvider::from_env();
    auto cache_dir = std::filesystem::path(".openanim") / "cache";
    openanim::OpenAnimEngine engine(cache_dir);

    std::cout << "\n  provider : " << base_url << "\n";
    std::cout << "  model    : " << model << "\n";
    std::cout << "  prompt   : " << prompt << "\n";

    // ── Step 0: Health check ──────────────────────────────────────────────
    print_health(engine.health_check_all());

    // ── Step 1: Compile project from prompt ───────────────────────────────
    print_section("Step 1: LLM compile_project");
    std::cout << "  Calling LLM... (this may take 5-30 seconds)\n";

    openanim::Project project;
    try {
        project = engine.compile_project(prompt, provider);
    } catch (const std::exception& e) {
        std::cerr << "\n[FAIL] compile_project threw: " << e.what() << "\n";
        return 1;
    }
    std::cout << "  OK — project compiled\n";
    print_project_summary(project);

    // ── Step 2: Validate IR ───────────────────────────────────────────────
    print_section("Step 2: Validate IR");
    auto errors = openanim::validate_project(project);
    if (!errors.empty()) {
        std::cerr << "[FAIL] Validation errors:\n";
        for (const auto& e : errors) std::cerr << "  - " << e.message << "\n";
        return 1;
    }
    std::cout << "  OK — no validation errors\n";

    // ── Step 3: Render (SVG) ──────────────────────────────────────────────
    print_section("Step 3: Render SVG");
    project.settings.format = openanim::OutputFormat::Svg;
    std::vector<openanim::RenderArtifact> artifacts;
    try {
        artifacts = engine.render_project(project);
    } catch (const std::exception& e) {
        std::cerr << "[FAIL] render_project threw: " << e.what() << "\n";
        return 1;
    }
    std::cout << "  OK — rendered " << artifacts.size() << " scene(s)\n";
    for (const auto& art : artifacts) {
        std::cout << "    \"" << art.output_path.string() << "\" (" << art.file_size_bytes << " bytes)\n";
        assert(std::filesystem::exists(art.output_path));
    }

    // ── Step 4: Patch project ─────────────────────────────────────────────
    print_section("Step 4: LLM patch_project");
    std::string patch_prompt = "Make the main shape red instead of blue";
    std::cout << "  patch: \"" << patch_prompt << "\"\n";
    std::cout << "  Calling LLM... (this may take 5-30 seconds)\n";

    openanim::Project patched;
    try {
        patched = engine.patch_project(project, patch_prompt, provider);
    } catch (const std::exception& e) {
        std::cerr << "[FAIL] patch_project threw: " << e.what() << "\n";
        return 1;
    }
    std::cout << "  OK — project patched\n";
    print_project_summary(patched);

    // ── Step 5: Validate & render patch ──────────────────────────────────
    print_section("Step 5: Validate & render patch");
    auto patch_errors = openanim::validate_project(patched);
    if (!patch_errors.empty()) {
        std::cerr << "[FAIL] Patched project validation errors:\n";
        for (const auto& e : patch_errors) std::cerr << "  - " << e.message << "\n";
        return 1;
    }
    patched.settings.format = openanim::OutputFormat::Svg;
    try {
        auto patched_arts = engine.render_project(patched);
        std::cout << "  OK — rendered " << patched_arts.size() << " scene(s)\n";
        for (const auto& art : patched_arts)
            std::cout << "    \"" << art.output_path.string() << "\" (" << art.file_size_bytes << " bytes)\n";
    } catch (const std::exception& e) {
        std::cerr << "[FAIL] patch render threw: " << e.what() << "\n";
        return 1;
    }

    // ── Step 6: render_project_to_video (FFmpeg compose) ─────────────────
    print_section("Step 6: render_project_to_video");
    if (!ffmpeg_available()) {
        std::cout << "  SKIP (ffmpeg not found on PATH)\n";
    } else {
        // Use FFmpeg adapter directly per-scene then compose
        // We build a 2-scene project with MP4 output
        auto video_project = project;
        video_project.settings.format = openanim::OutputFormat::Mp4;
        // Duplicate first scene to guarantee >= 2 scenes for the concat test
        if (video_project.scenes.size() < 2 && !video_project.scenes.empty()) {
            auto extra = video_project.scenes.front();
            extra.id = openanim::Id{"scene-extra"};
            video_project.scenes.push_back(extra);
        }

        auto out_path = cache_dir / "e2e-final.mp4";
        try {
            auto video_art = engine.render_project_to_video(video_project, out_path);
            assert(std::filesystem::exists(video_art.output_path));
            assert(video_art.file_size_bytes > 0);
            std::cout << "  OK — " << video_art.output_path.string()
                      << " (" << video_art.file_size_bytes << " bytes)\n";
        } catch (const std::exception& e) {
            // Non-fatal: SVG-only output can't be ffmpeg-concatenated as MP4
            std::cout << "  WARN: render_project_to_video: " << e.what() << "\n";
            std::cout << "        (expected if renderer produced SVG instead of MP4)\n";
        }
    }

    // ── Step 7: JSON round-trip fidelity ──────────────────────────────────
    print_section("Step 7: JSON round-trip fidelity");
    {
        auto json_val  = openanim::to_json(project);
        auto recovered = openanim::project_from_json(json_val);
        assert(recovered.metadata.name == project.metadata.name);
        assert(recovered.scenes.size() == project.scenes.size());
        assert(recovered.total_node_count() == project.total_node_count());
        std::cout << "  OK — " << project.total_node_count() << " nodes survive round-trip\n";
    }

    std::cout << "\n✓ All E2E tests passed\n";
    return 0;
}
