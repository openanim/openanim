#include "openanim/hasher.hpp"

#include <boost/json/serialize.hpp>

#include <array>
#include <iomanip>
#include <sstream>

namespace openanim {
namespace {

std::uint64_t fnv1a(std::string_view data, std::uint64_t seed) {
    std::uint64_t hash = 14695981039346656037ull ^ seed;
    for (unsigned char c : data) {
        hash ^= c;
        hash *= 1099511628211ull;
    }
    return hash;
}

std::string hex64(std::uint64_t value) {
    std::ostringstream out;
    out << std::hex << std::setw(16) << std::setfill('0') << value;
    return out.str();
}

}  // namespace

std::string hash_bytes(std::string_view data) {
    std::array<std::uint64_t, 4> parts{
        fnv1a(data, 0x00),
        fnv1a(data, 0x9e3779b97f4a7c15ull),
        fnv1a(data, 0xbf58476d1ce4e5b9ull),
        fnv1a(data, 0x94d049bb133111ebull),
    };
    return hex64(parts[0]) + hex64(parts[1]) + hex64(parts[2]) + hex64(parts[3]);
}

std::string hash_json(const Json& json) {
    return hash_bytes(boost::json::serialize(json));
}

std::string hash_node(const Node& node) {
    Object out{{"id", to_json(node.id)}, {"node_type", to_string(node.node_type)}, {"components", to_json(node.components)}};
    if (node.name) out["name"] = *node.name;
    return hash_json(out);
}

std::string hash_node_with_children(const Node& node) {
    return hash_json(to_json(node));
}

std::string hash_scene(const Scene& scene) {
    return hash_json(to_json(scene));
}

std::string hash_project(const Project& project) {
    return hash_json(to_json(project));
}

std::string hash_render_config(const RenderSettings& settings) {
    return hash_json(to_json(settings));
}

std::string cache_key(std::string_view scene_hash, std::string_view render_config_hash, std::string_view provider_name) {
    std::string combined;
    combined.reserve(scene_hash.size() + render_config_hash.size() + provider_name.size() + 2);
    combined.append(provider_name);
    combined.push_back(':');
    combined.append(scene_hash);
    combined.push_back(':');
    combined.append(render_config_hash);
    return hash_bytes(combined);
}

}  // namespace openanim
