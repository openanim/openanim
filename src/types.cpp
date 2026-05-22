#include "openanim/types.hpp"

#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>
#include <stdexcept>

namespace openanim {
namespace {

template <typename Enum>
Enum enum_from_string(std::string_view value, std::initializer_list<std::pair<std::string_view, Enum>> values, Enum fallback) {
    for (const auto& [name, enum_value] : values) {
        if (name == value) {
            return enum_value;
        }
    }
    return fallback;
}

template <typename Enum>
std::string enum_to_string(Enum value, std::initializer_list<std::pair<Enum, std::string_view>> values) {
    for (const auto& [enum_value, name] : values) {
        if (enum_value == value) {
            return std::string{name};
        }
    }
    return "unknown";
}

std::optional<std::string> optional_string(const Object& object, std::string_view key) {
    const auto* value = find(object, key);
    if (!value || value->is_null()) {
        return std::nullopt;
    }
    return std::string(value->as_string());
}

}  // namespace

Id Id::make() {
    static thread_local boost::uuids::random_generator generator;
    return {boost::uuids::to_string(generator())};
}

Id Id::nil() {
    return {"00000000-0000-0000-0000-000000000000"};
}

Vec2 Vec2::zero() { return {}; }
Vec2 Vec2::one() { return {1.0, 1.0}; }
Vec2 Vec2::half() { return {0.5, 0.5}; }

Color Color::white() { return {1.0, 1.0, 1.0, 1.0}; }
Color Color::black() { return {0.0, 0.0, 0.0, 1.0}; }
Color Color::transparent() { return {0.0, 0.0, 0.0, 0.0}; }

std::optional<Color> Color::from_hex(std::string hex) {
    if (!hex.empty() && hex.front() == '#') {
        hex.erase(hex.begin());
    }
    if (hex.size() != 6 && hex.size() != 8) {
        return std::nullopt;
    }

    auto component = [&](std::size_t offset) -> std::optional<double> {
        try {
            return static_cast<double>(std::stoi(hex.substr(offset, 2), nullptr, 16)) / 255.0;
        } catch (...) {
            return std::nullopt;
        }
    };

    auto r = component(0);
    auto g = component(2);
    auto b = component(4);
    if (!r || !g || !b) {
        return std::nullopt;
    }
    auto a = hex.size() == 8 ? component(6) : std::optional<double>{1.0};
    if (!a) {
        return std::nullopt;
    }
    return Color{*r, *g, *b, *a};
}

DurationSecs DurationSecs::from_frames(std::uint64_t frames, double fps) {
    return {static_cast<double>(frames) / fps};
}

std::uint64_t DurationSecs::to_frames(double fps) const {
    return static_cast<std::uint64_t>(std::llround(value * fps));
}

std::string to_string(LineCap value) { return enum_to_string(value, {{LineCap::Butt, "butt"}, {LineCap::Round, "round"}, {LineCap::Square, "square"}}); }
std::string to_string(LineJoin value) { return enum_to_string(value, {{LineJoin::Miter, "miter"}, {LineJoin::Round, "round"}, {LineJoin::Bevel, "bevel"}}); }
std::string to_string(FontWeight value) { return enum_to_string(value, {{FontWeight::Thin, "thin"}, {FontWeight::Light, "light"}, {FontWeight::Regular, "regular"}, {FontWeight::Medium, "medium"}, {FontWeight::SemiBold, "semi_bold"}, {FontWeight::Bold, "bold"}, {FontWeight::ExtraBold, "extra_bold"}, {FontWeight::Black, "black"}}); }
std::string to_string(FontStyle value) { return enum_to_string(value, {{FontStyle::Normal, "normal"}, {FontStyle::Italic, "italic"}, {FontStyle::Oblique, "oblique"}}); }
std::string to_string(TextAlign value) { return enum_to_string(value, {{TextAlign::Left, "left"}, {TextAlign::Center, "center"}, {TextAlign::Right, "right"}}); }
std::string to_string(TextVerticalAlign value) { return enum_to_string(value, {{TextVerticalAlign::Top, "top"}, {TextVerticalAlign::Middle, "middle"}, {TextVerticalAlign::Bottom, "bottom"}}); }
std::string to_string(ImageFit value) { return enum_to_string(value, {{ImageFit::Contain, "contain"}, {ImageFit::Cover, "cover"}, {ImageFit::Fill, "fill"}, {ImageFit::None, "none"}}); }
std::string to_string(OutputFormat value) { return enum_to_string(value, {{OutputFormat::Mp4, "mp4"}, {OutputFormat::Webm, "webm"}, {OutputFormat::Gif, "gif"}, {OutputFormat::Png, "png"}, {OutputFormat::Svg, "svg"}, {OutputFormat::ImageSequence, "image_sequence"}}); }

LineCap line_cap_from_string(std::string_view value) { return enum_from_string(value, {{"butt", LineCap::Butt}, {"round", LineCap::Round}, {"square", LineCap::Square}}, LineCap::Butt); }
LineJoin line_join_from_string(std::string_view value) { return enum_from_string(value, {{"miter", LineJoin::Miter}, {"round", LineJoin::Round}, {"bevel", LineJoin::Bevel}}, LineJoin::Miter); }
FontWeight font_weight_from_string(std::string_view value) { return enum_from_string(value, {{"thin", FontWeight::Thin}, {"light", FontWeight::Light}, {"regular", FontWeight::Regular}, {"medium", FontWeight::Medium}, {"semi_bold", FontWeight::SemiBold}, {"bold", FontWeight::Bold}, {"extra_bold", FontWeight::ExtraBold}, {"black", FontWeight::Black}}, FontWeight::Regular); }
FontStyle font_style_from_string(std::string_view value) { return enum_from_string(value, {{"normal", FontStyle::Normal}, {"italic", FontStyle::Italic}, {"oblique", FontStyle::Oblique}}, FontStyle::Normal); }
TextAlign text_align_from_string(std::string_view value) { return enum_from_string(value, {{"left", TextAlign::Left}, {"center", TextAlign::Center}, {"right", TextAlign::Right}}, TextAlign::Left); }
TextVerticalAlign text_vertical_align_from_string(std::string_view value) { return enum_from_string(value, {{"top", TextVerticalAlign::Top}, {"middle", TextVerticalAlign::Middle}, {"bottom", TextVerticalAlign::Bottom}}, TextVerticalAlign::Top); }
ImageFit image_fit_from_string(std::string_view value) { return enum_from_string(value, {{"contain", ImageFit::Contain}, {"cover", ImageFit::Cover}, {"fill", ImageFit::Fill}, {"none", ImageFit::None}}, ImageFit::Contain); }
OutputFormat output_format_from_string(std::string_view value) { return enum_from_string(value, {{"mp4", OutputFormat::Mp4}, {"webm", OutputFormat::Webm}, {"gif", OutputFormat::Gif}, {"png", OutputFormat::Png}, {"svg", OutputFormat::Svg}, {"image_sequence", OutputFormat::ImageSequence}}, OutputFormat::Mp4); }

Json to_json(const Id& id) { return boost::json::string(id.value); }

Json to_json(const Vec2& vec) {
    return Object{{"x", vec.x}, {"y", vec.y}};
}

Json to_json(const Vec3& vec) {
    return Object{{"x", vec.x}, {"y", vec.y}, {"z", vec.z}};
}

Json to_json(const Color& color) {
    return Object{{"r", color.r}, {"g", color.g}, {"b", color.b}, {"a", color.a}};
}

Json to_json(const Stroke& stroke) {
    return Object{{"color", to_json(stroke.color)}, {"width", stroke.width}, {"line_cap", to_string(stroke.line_cap)}, {"line_join", to_string(stroke.line_join)}};
}

Json to_json(const FontSpec& font) {
    return Object{{"family", boost::json::string(font.family)}, {"size", font.size}, {"weight", to_string(font.weight).c_str()}, {"style", to_string(font.style).c_str()}};
}

Json to_json(const AssetRef& asset) {
    Object out{{"asset_id", boost::json::string(asset.asset_id)}, {"path", boost::json::string(asset.path)}};
    if (asset.mime_type) out["mime_type"] = boost::json::string(*asset.mime_type);
    if (asset.content_hash) out["content_hash"] = boost::json::string(*asset.content_hash);
    return out;
}

Json to_json(const DurationSecs& duration) {
    return duration.value;
}

Id id_from_json(const Json& json) {
    if (json.is_string()) {
        return {std::string(json.as_string())};
    }
    return Id::nil();
}

Vec2 vec2_from_json(const Json& json, Vec2 fallback) {
    if (!json.is_object()) {
        return fallback;
    }
    const auto& object = json.as_object();
    return {number_or(object, "x", fallback.x), number_or(object, "y", fallback.y)};
}

Color color_from_json(const Json& json, Color fallback) {
    if (!json.is_object()) {
        return fallback;
    }
    const auto& object = json.as_object();
    return {
        number_or(object, "r", fallback.r),
        number_or(object, "g", fallback.g),
        number_or(object, "b", fallback.b),
        number_or(object, "a", fallback.a),
    };
}

Stroke stroke_from_json(const Json& json) {
    Stroke stroke;
    if (!json.is_object()) return stroke;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "color")) stroke.color = color_from_json(*value, stroke.color);
    stroke.width = number_or(object, "width", stroke.width);
    if (auto cap = optional_string(object, "line_cap")) stroke.line_cap = line_cap_from_string(*cap);
    if (auto join = optional_string(object, "line_join")) stroke.line_join = line_join_from_string(*join);
    return stroke;
}

FontSpec font_from_json(const Json& json) {
    FontSpec font;
    if (!json.is_object()) return font;
    const auto& object = json.as_object();
    font.family = string_or(object, "family", font.family);
    font.size = number_or(object, "size", font.size);
    if (auto weight = optional_string(object, "weight")) font.weight = font_weight_from_string(*weight);
    if (auto style = optional_string(object, "style")) font.style = font_style_from_string(*style);
    return font;
}

AssetRef asset_from_json(const Json& json) {
    AssetRef asset;
    if (!json.is_object()) return asset;
    const auto& object = json.as_object();
    asset.asset_id = string_or(object, "asset_id");
    asset.path = string_or(object, "path");
    asset.mime_type = optional_string(object, "mime_type");
    asset.content_hash = optional_string(object, "content_hash");
    return asset;
}

DurationSecs duration_from_json(const Json& json) {
    if (json.is_number()) {
        return {json.to_number<double>()};
    }
    return {};
}

const Json* find(const Object& object, std::string_view key) {
    auto it = object.find(key);
    if (it == object.end()) {
        return nullptr;
    }
    return &it->value();
}

std::string string_or(const Object& object, std::string_view key, std::string fallback) {
    const auto* value = find(object, key);
    if (!value || !value->is_string()) {
        return fallback;
    }
    return std::string(value->as_string());
}

double number_or(const Object& object, std::string_view key, double fallback) {
    const auto* value = find(object, key);
    if (!value || !value->is_number()) {
        return fallback;
    }
    return value->to_number<double>();
}

bool bool_or(const Object& object, std::string_view key, bool fallback) {
    const auto* value = find(object, key);
    if (!value || !value->is_bool()) {
        return fallback;
    }
    return value->as_bool();
}

}  // namespace openanim
