#pragma once

#include "openanim/scene_ir.hpp"

#include <string>
#include <string_view>

namespace openanim {

std::string hash_bytes(std::string_view data);
std::string hash_json(const Json& json);
std::string hash_node(const Node& node);
std::string hash_node_with_children(const Node& node);
std::string hash_scene(const Scene& scene);
std::string hash_project(const Project& project);
std::string hash_render_config(const RenderSettings& settings);
std::string cache_key(std::string_view scene_hash, std::string_view render_config_hash, std::string_view provider_name);

}  // namespace openanim
