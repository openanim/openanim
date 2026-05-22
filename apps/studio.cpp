#include "openanim/engine.hpp"

#include <filesystem>
#include <iostream>

int main(int argc, char** argv) {
    try {
        std::filesystem::path project_path = argc > 1 ? argv[1] : "project.json";
        std::filesystem::path cache_dir = ".openanim/cache";

        auto project = openanim::load_project_file(project_path);
        openanim::OpenAnimEngine engine(cache_dir);

        auto errors = engine.validate_project(project);
        if (!errors.empty()) {
            std::cerr << "Project validation failed:\n";
            for (const auto& error : errors) {
                std::cerr << "  - " << error.message << '\n';
            }
            return 1;
        }

        auto artifacts = engine.render_project(project);
        std::cout << "Rendered " << artifacts.size() << " scene(s)\n";
        for (const auto& artifact : artifacts) {
            std::cout << "  " << artifact.output_path << " (" << artifact.file_size_bytes << " bytes)\n";
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "openanim-studio: " << error.what() << '\n';
        return 1;
    }
}
