#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <vector>

namespace openanim {

// Result of running a subprocess
struct SubprocessResult {
    int exit_code = 0;
    std::string stdout_text;
    std::string stderr_text;
};

// How to execute renderer binaries
enum class SandboxMode {
    DirectProcess,  // Run binary directly (local dev / testing)
    Docker,         // Run binary inside a Docker container per provider
};

// Configuration for the execution sandbox
struct SandboxConfig {
    SandboxMode mode = SandboxMode::DirectProcess;

    // Docker-specific options (only used when mode == Docker)
    std::optional<std::string> docker_image;
    std::optional<std::string> docker_network;
    std::optional<std::uint64_t> timeout_seconds;
    std::optional<std::uint64_t> memory_limit_mb;
};

// Run a command with the given arguments in the specified working directory.
// In DirectProcess mode: executes the binary directly.
// In Docker mode: wraps in `docker run --rm -v workdir:/workspace`.
SubprocessResult run_command(
    const std::string& program,
    const std::vector<std::string>& args,
    const std::filesystem::path& working_dir,
    const SandboxConfig& config = {});

// Returns true if the given binary name is on the PATH
bool binary_exists(const std::string& binary_name);

}  // namespace openanim
