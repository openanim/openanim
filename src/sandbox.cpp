#include "openanim/sandbox.hpp"

#include <atomic>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <stdexcept>

#ifdef _WIN32
#include <process.h>
#define getpid _getpid
#else
#include <sys/wait.h>
#include <unistd.h>
#endif

namespace openanim {
namespace {

static std::string read_file(const std::filesystem::path& path) {
    std::ifstream f(path);
    if (!f) return {};
    std::ostringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

// Wrap a string in single quotes, escaping any embedded single quotes
static std::string shell_quote(const std::string& s) {
    std::string out = "'";
    for (char c : s) {
        if (c == '\'') out += "'\\''";
        else out += c;
    }
    out += "'";
    return out;
}

// Build a shell command string for DirectProcess mode
static std::string build_direct_command(
    const std::string& program,
    const std::vector<std::string>& args,
    const std::filesystem::path& working_dir)
{
    std::string cmd = "cd " + shell_quote(working_dir.string()) + " && " + program;
    for (const auto& arg : args) cmd += " " + shell_quote(arg);
    return cmd;
}

// Build a shell command string for Docker mode
static std::string build_docker_command(
    const std::string& program,
    const std::vector<std::string>& args,
    const std::filesystem::path& working_dir,
    const SandboxConfig& config)
{
    std::string docker = "docker run --rm";
    docker += " -v " + shell_quote(working_dir.string()) + ":/workspace";
    docker += " -w /workspace";
    if (config.memory_limit_mb) {
        docker += " --memory=" + std::to_string(*config.memory_limit_mb) + "m";
    }
    if (config.docker_network) {
        docker += " --network=" + *config.docker_network;
    }
    docker += " " + *config.docker_image;
    docker += " " + program;
    for (const auto& arg : args) docker += " " + shell_quote(arg);
    return docker;
}

static int get_exit_code(int raw) {
#ifdef _WIN32
    return raw;
#else
    if (raw == -1) return -1;
    return WIFEXITED(raw) ? WEXITSTATUS(raw) : -1;
#endif
}

}  // namespace

SubprocessResult run_command(
    const std::string& program,
    const std::vector<std::string>& args,
    const std::filesystem::path& working_dir,
    const SandboxConfig& config)
{
    std::filesystem::create_directories(working_dir);

    // Unique temp file names per call using PID + static counter
    static std::atomic<int> counter{0};
    auto tag = std::to_string(static_cast<long>(getpid())) + "_" + std::to_string(counter++);
    auto stdout_file = std::filesystem::temp_directory_path() / ("oa_out_" + tag + ".txt");
    auto stderr_file = std::filesystem::temp_directory_path() / ("oa_err_" + tag + ".txt");

    std::string cmd;
    if (config.mode == SandboxMode::Docker && config.docker_image) {
        cmd = build_docker_command(program, args, working_dir, config);
    } else {
        cmd = build_direct_command(program, args, working_dir);
    }
    cmd += " 1>" + shell_quote(stdout_file.string());
    cmd += " 2>" + shell_quote(stderr_file.string());

    int raw = std::system(cmd.c_str());

    SubprocessResult result;
    result.exit_code = get_exit_code(raw);
    result.stdout_text = read_file(stdout_file);
    result.stderr_text = read_file(stderr_file);

    std::filesystem::remove(stdout_file);
    std::filesystem::remove(stderr_file);
    return result;
}

bool binary_exists(const std::string& binary_name) {
    // Use `which` on POSIX, `where` on Windows
#ifdef _WIN32
    std::string cmd = "where " + binary_name + " >NUL 2>&1";
#else
    std::string cmd = "which " + binary_name + " >/dev/null 2>&1";
#endif
    return std::system(cmd.c_str()) == 0;
}

}  // namespace openanim
