#pragma once

#include <boost/json.hpp>
#include <boost/uuid/random_generator.hpp>
#include <boost/uuid/string_generator.hpp>
#include <boost/uuid/uuid.hpp>
#include <boost/uuid/uuid_io.hpp>

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace openanim {

using Json = boost::json::value;
using Object = boost::json::object;
using Array = boost::json::array;

struct Id {
    std::string value;

    static Id make();
    static Id nil();

    friend bool operator==(const Id&, const Id&) = default;
};

using NodeId = Id;
using SceneId = Id;
using ProjectId = Id;
using EventId = Id;
using ContentHash = std::string;

struct Vec2 {
    double x = 0.0;
    double y = 0.0;

    static Vec2 zero();
    static Vec2 one();
    static Vec2 half();
};

struct Vec3 {
    double x = 0.0;
    double y = 0.0;
    double z = 0.0;
};

struct Color {
    double r = 1.0;
    double g = 1.0;
    double b = 1.0;
    double a = 1.0;

    static Color white();
    static Color black();
    static Color transparent();
    static std::optional<Color> from_hex(std::string hex);
};

enum class LineCap { Butt, Round, Square };
enum class LineJoin { Miter, Round, Bevel };
enum class FontWeight { Thin, Light, Regular, Medium, SemiBold, Bold, ExtraBold, Black };
enum class FontStyle { Normal, Italic, Oblique };
enum class TextAlign { Left, Center, Right };
enum class TextVerticalAlign { Top, Middle, Bottom };
enum class ImageFit { Contain, Cover, Fill, None };
enum class OutputFormat { Mp4, Webm, Gif, Png, Svg, ImageSequence };

struct Stroke {
    Color color = Color::white();
    double width = 1.0;
    LineCap line_cap = LineCap::Butt;
    LineJoin line_join = LineJoin::Miter;
};

struct FontSpec {
    std::string family = "Inter";
    double size = 24.0;
    FontWeight weight = FontWeight::Regular;
    FontStyle style = FontStyle::Normal;
};

struct AssetRef {
    std::string asset_id;
    std::string path;
    std::optional<std::string> mime_type;
    std::optional<ContentHash> content_hash;
};

struct DurationSecs {
    double value = 0.0;

    static DurationSecs from_frames(std::uint64_t frames, double fps);
    std::uint64_t to_frames(double fps) const;
};

std::string to_string(LineCap value);
std::string to_string(LineJoin value);
std::string to_string(FontWeight value);
std::string to_string(FontStyle value);
std::string to_string(TextAlign value);
std::string to_string(TextVerticalAlign value);
std::string to_string(ImageFit value);
std::string to_string(OutputFormat value);

LineCap line_cap_from_string(std::string_view value);
LineJoin line_join_from_string(std::string_view value);
FontWeight font_weight_from_string(std::string_view value);
FontStyle font_style_from_string(std::string_view value);
TextAlign text_align_from_string(std::string_view value);
TextVerticalAlign text_vertical_align_from_string(std::string_view value);
ImageFit image_fit_from_string(std::string_view value);
OutputFormat output_format_from_string(std::string_view value);

Json to_json(const Id& id);
Json to_json(const Vec2& vec);
Json to_json(const Vec3& vec);
Json to_json(const Color& color);
Json to_json(const Stroke& stroke);
Json to_json(const FontSpec& font);
Json to_json(const AssetRef& asset);
Json to_json(const DurationSecs& duration);

Id id_from_json(const Json& json);
Vec2 vec2_from_json(const Json& json, Vec2 fallback = {});
Color color_from_json(const Json& json, Color fallback = Color::white());
Stroke stroke_from_json(const Json& json);
FontSpec font_from_json(const Json& json);
AssetRef asset_from_json(const Json& json);
DurationSecs duration_from_json(const Json& json);

const Json* find(const Object& object, std::string_view key);
std::string string_or(const Object& object, std::string_view key, std::string fallback = {});
double number_or(const Object& object, std::string_view key, double fallback = 0.0);
bool bool_or(const Object& object, std::string_view key, bool fallback = false);

}  // namespace openanim
