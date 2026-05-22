#include "openanim/scene_ir.hpp"

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <numeric>
#include <stdexcept>

namespace openanim {
namespace {

boost::json::string js(std::string_view value) {
    return boost::json::string(value);
}

template <typename Enum>
Enum enum_from_string(std::string_view value, std::initializer_list<std::pair<std::string_view, Enum>> values, Enum fallback) {
    for (const auto& [name, enum_value] : values) {
        if (name == value) return enum_value;
    }
    return fallback;
}

template <typename Enum>
std::string enum_to_string(Enum value, std::initializer_list<std::pair<Enum, std::string_view>> values) {
    for (const auto& [enum_value, name] : values) {
        if (enum_value == value) return std::string{name};
    }
    return "unknown";
}

std::optional<std::string> opt_string(const Object& object, std::string_view key) {
    const auto* value = find(object, key);
    if (!value || !value->is_string()) return std::nullopt;
    return std::string(value->as_string());
}

std::vector<std::string> strings_from_json(const Json& json) {
    std::vector<std::string> values;
    if (!json.is_array()) return values;
    for (const auto& item : json.as_array()) {
        if (item.is_string()) values.emplace_back(item.as_string());
    }
    return values;
}

std::vector<Id> ids_from_json(const Json& json) {
    std::vector<Id> values;
    if (!json.is_array()) return values;
    for (const auto& item : json.as_array()) values.push_back(id_from_json(item));
    return values;
}

Array ids_to_json(const std::vector<Id>& ids) {
    Array out;
    for (const auto& id : ids) out.push_back(to_json(id));
    return out;
}

Array vec2s_to_json(const std::vector<Vec2>& values) {
    Array out;
    for (const auto& value : values) out.push_back(to_json(value));
    return out;
}

std::vector<Vec2> vec2s_from_json(const Json& json) {
    std::vector<Vec2> out;
    if (!json.is_array()) return out;
    for (const auto& value : json.as_array()) out.push_back(vec2_from_json(value));
    return out;
}

Json property_value_to_json(const PropertyValue& value) {
    Object out;
    if (std::holds_alternative<double>(value)) {
        out["type"] = "scalar";
        out["value"] = std::get<double>(value);
    } else if (std::holds_alternative<Vec2>(value)) {
        out["type"] = "vec2";
        out["value"] = to_json(std::get<Vec2>(value));
    } else {
        out["type"] = "color";
        out["value"] = to_json(std::get<Color>(value));
    }
    return out;
}

PropertyValue property_value_from_json(const Json& json) {
    if (!json.is_object()) return 0.0;
    const auto& object = json.as_object();
    auto type = string_or(object, "type", "scalar");
    const auto* value = find(object, "value");
    if (type == "vec2" && value) return vec2_from_json(*value);
    if (type == "color" && value) return color_from_json(*value);
    if (value && value->is_number()) return value->to_number<double>();
    return 0.0;
}

}  // namespace

Transform Transform::at(double x, double y) {
    Transform transform;
    transform.position = {x, y};
    return transform;
}

Style Style::with_fill(Color color) {
    Style style;
    style.fill = color;
    return style;
}

Style Style::with_stroke(Stroke stroke) {
    Style style;
    style.stroke = stroke;
    return style;
}

Node Node::make(NodeType type) {
    Node node;
    node.node_type = type;
    return node;
}

Node Node::named(std::string name, NodeType type) {
    Node node = make(type);
    node.name = std::move(name);
    return node;
}

bool Node::is_renderable() const {
    return components.shape || components.text || components.math || components.image || components.code || components.diagram;
}

bool Node::is_leaf() const { return children.empty(); }
bool Node::is_root() const { return !parent.has_value(); }

std::vector<const AnimationEvent*> Timeline::sorted_events() const {
    std::vector<const AnimationEvent*> out;
    for (const auto& event : events) out.push_back(&event);
    std::sort(out.begin(), out.end(), [](const auto* a, const auto* b) {
        return a->start_time.value < b->start_time.value;
    });
    return out;
}

Scene Scene::make(std::string name) {
    auto root = Node::named("root", NodeType::Group);
    auto root_id = root.id;
    Scene scene;
    scene.name = std::move(name);
    scene.root_node = root_id;
    scene.nodes.push_back(std::move(root));
    return scene;
}

const Node* Scene::find_node(const NodeId& id) const {
    auto it = std::find_if(nodes.begin(), nodes.end(), [&](const Node& node) { return node.id == id; });
    return it == nodes.end() ? nullptr : &*it;
}

Node* Scene::find_node(const NodeId& id) {
    auto it = std::find_if(nodes.begin(), nodes.end(), [&](const Node& node) { return node.id == id; });
    return it == nodes.end() ? nullptr : &*it;
}

const Node* Scene::find_node_by_name(std::string_view name) const {
    auto it = std::find_if(nodes.begin(), nodes.end(), [&](const Node& node) {
        return node.name && *node.name == name;
    });
    return it == nodes.end() ? nullptr : &*it;
}

std::size_t Scene::node_count() const { return nodes.size(); }

const AssetRef* AssetManifest::find_asset(std::string_view asset_id) const {
    auto it = std::find_if(assets.begin(), assets.end(), [&](const AssetRef& asset) { return asset.asset_id == asset_id; });
    return it == assets.end() ? nullptr : &*it;
}

void AssetManifest::add_asset(AssetRef asset) { assets.push_back(std::move(asset)); }

Project Project::make(std::string name) {
    Project project;
    project.metadata.name = std::move(name);
    return project;
}

void Project::add_scene(Scene scene) { scenes.push_back(std::move(scene)); }
std::size_t Project::scene_count() const { return scenes.size(); }
std::size_t Project::total_node_count() const {
    return std::accumulate(scenes.begin(), scenes.end(), std::size_t{0}, [](auto total, const Scene& scene) {
        return total + scene.node_count();
    });
}

std::string to_string(NodeType value) { return enum_to_string(value, {{NodeType::Group, "group"}, {NodeType::Shape, "shape"}, {NodeType::Text, "text"}, {NodeType::Math, "math"}, {NodeType::Image, "image"}, {NodeType::Code, "code"}, {NodeType::Diagram, "diagram"}, {NodeType::Custom, "custom"}}); }
std::string to_string(ShapeType value) { return enum_to_string(value, {{ShapeType::Rectangle, "rectangle"}, {ShapeType::Circle, "circle"}, {ShapeType::Ellipse, "ellipse"}, {ShapeType::Line, "line"}, {ShapeType::Polygon, "polygon"}, {ShapeType::Path, "path"}, {ShapeType::Arc, "arc"}, {ShapeType::Arrow, "arrow"}}); }
std::string to_string(DiagramLanguage value) { return enum_to_string(value, {{DiagramLanguage::Mermaid, "mermaid"}, {DiagramLanguage::PlantUml, "plant_uml"}, {DiagramLanguage::Graphviz, "graphviz"}, {DiagramLanguage::D2, "d2"}}); }
std::string to_string(AnimatableProperty value) { return enum_to_string(value, {{AnimatableProperty::PositionX, "position_x"}, {AnimatableProperty::PositionY, "position_y"}, {AnimatableProperty::Position, "position"}, {AnimatableProperty::Rotation, "rotation"}, {AnimatableProperty::ScaleX, "scale_x"}, {AnimatableProperty::ScaleY, "scale_y"}, {AnimatableProperty::Scale, "scale"}, {AnimatableProperty::Opacity, "opacity"}, {AnimatableProperty::FillColor, "fill_color"}, {AnimatableProperty::StrokeColor, "stroke_color"}, {AnimatableProperty::StrokeWidth, "stroke_width"}}); }
std::string to_string(EasingFunction value) { return enum_to_string(value, {{EasingFunction::Linear, "linear"}, {EasingFunction::EaseIn, "ease_in"}, {EasingFunction::EaseOut, "ease_out"}, {EasingFunction::EaseInOut, "ease_in_out"}, {EasingFunction::EaseInQuad, "ease_in_quad"}, {EasingFunction::EaseOutQuad, "ease_out_quad"}, {EasingFunction::EaseInOutQuad, "ease_in_out_quad"}, {EasingFunction::Step, "step"}}); }
std::string to_string(EventType value) { return enum_to_string(value, {{EventType::FadeIn, "fade_in"}, {EventType::FadeOut, "fade_out"}, {EventType::Create, "create"}, {EventType::Write, "write"}, {EventType::Transform, "transform"}, {EventType::MoveTo, "move_to"}, {EventType::Rotate, "rotate"}, {EventType::ScaleTo, "scale_to"}, {EventType::Custom, "custom"}}); }

NodeType node_type_from_string(std::string_view value) { return enum_from_string(value, {{"group", NodeType::Group}, {"shape", NodeType::Shape}, {"text", NodeType::Text}, {"math", NodeType::Math}, {"image", NodeType::Image}, {"code", NodeType::Code}, {"diagram", NodeType::Diagram}, {"custom", NodeType::Custom}}, NodeType::Custom); }
ShapeType shape_type_from_string(std::string_view value) { return enum_from_string(value, {{"rectangle", ShapeType::Rectangle}, {"circle", ShapeType::Circle}, {"ellipse", ShapeType::Ellipse}, {"line", ShapeType::Line}, {"polygon", ShapeType::Polygon}, {"path", ShapeType::Path}, {"arc", ShapeType::Arc}, {"arrow", ShapeType::Arrow}}, ShapeType::Circle); }
DiagramLanguage diagram_language_from_string(std::string_view value) { return enum_from_string(value, {{"mermaid", DiagramLanguage::Mermaid}, {"plant_uml", DiagramLanguage::PlantUml}, {"graphviz", DiagramLanguage::Graphviz}, {"d2", DiagramLanguage::D2}}, DiagramLanguage::Mermaid); }
AnimatableProperty animatable_property_from_string(std::string_view value) { return enum_from_string(value, {{"position_x", AnimatableProperty::PositionX}, {"position_y", AnimatableProperty::PositionY}, {"position", AnimatableProperty::Position}, {"rotation", AnimatableProperty::Rotation}, {"scale_x", AnimatableProperty::ScaleX}, {"scale_y", AnimatableProperty::ScaleY}, {"scale", AnimatableProperty::Scale}, {"opacity", AnimatableProperty::Opacity}, {"fill_color", AnimatableProperty::FillColor}, {"stroke_color", AnimatableProperty::StrokeColor}, {"stroke_width", AnimatableProperty::StrokeWidth}}, AnimatableProperty::Opacity); }
EasingFunction easing_from_string(std::string_view value) { return enum_from_string(value, {{"linear", EasingFunction::Linear}, {"ease_in", EasingFunction::EaseIn}, {"ease_out", EasingFunction::EaseOut}, {"ease_in_out", EasingFunction::EaseInOut}, {"ease_in_quad", EasingFunction::EaseInQuad}, {"ease_out_quad", EasingFunction::EaseOutQuad}, {"ease_in_out_quad", EasingFunction::EaseInOutQuad}, {"step", EasingFunction::Step}}, EasingFunction::EaseInOut); }
EventType event_type_from_string(std::string_view value) { return enum_from_string(value, {{"fade_in", EventType::FadeIn}, {"fade_out", EventType::FadeOut}, {"create", EventType::Create}, {"write", EventType::Write}, {"transform", EventType::Transform}, {"move_to", EventType::MoveTo}, {"rotate", EventType::Rotate}, {"scale_to", EventType::ScaleTo}, {"custom", EventType::Custom}}, EventType::Custom); }

Json to_json(const Transform& transform) {
    return Object{{"position", to_json(transform.position)}, {"rotation", transform.rotation}, {"scale", to_json(transform.scale)}, {"anchor", to_json(transform.anchor)}};
}

Json to_json(const Style& style) {
    Object out{{"opacity", style.opacity}, {"z_index", style.z_index}, {"visible", style.visible}};
    if (style.fill) out["fill"] = to_json(*style.fill);
    if (style.stroke) out["stroke"] = to_json(*style.stroke);
    return out;
}

Json to_json(const Shape& shape) {
    Object kind{{"type", js(to_string(shape.kind.type))}};
    const auto& k = shape.kind;
    switch (k.type) {
        case ShapeType::Rectangle: kind["width"] = k.width; kind["height"] = k.height; kind["corner_radius"] = k.corner_radius; break;
        case ShapeType::Circle: kind["radius"] = k.radius; break;
        case ShapeType::Ellipse: kind["rx"] = k.rx; kind["ry"] = k.ry; break;
        case ShapeType::Line: kind["start"] = to_json(k.start); kind["end"] = to_json(k.end); break;
        case ShapeType::Polygon: kind["points"] = vec2s_to_json(k.points); kind["closed"] = k.closed; break;
        case ShapeType::Path: kind["data"] = k.data; break;
        case ShapeType::Arc: kind["radius"] = k.radius; kind["start_angle"] = k.start_angle; kind["end_angle"] = k.end_angle; break;
        case ShapeType::Arrow: kind["start"] = to_json(k.start); kind["end"] = to_json(k.end); kind["head_size"] = k.head_size; break;
    }
    return Object{{"kind", kind}};
}

Json to_json(const TextContent& text) {
    Object out{{"content", js(text.content)}, {"font", to_json(text.font)}, {"align", js(to_string(text.align))}, {"vertical_align", js(to_string(text.vertical_align))}, {"line_height", text.line_height}};
    if (text.max_width) out["max_width"] = *text.max_width;
    return out;
}

Json to_json(const MathExpression& math) { return Object{{"latex", js(math.latex)}, {"font_size", math.font_size}, {"color", to_json(math.color)}}; }
Json to_json(const ImageContent& image) {
    Object out{{"asset_ref", to_json(image.asset_ref)}, {"fit", js(to_string(image.fit))}};
    if (image.width) out["width"] = *image.width;
    if (image.height) out["height"] = *image.height;
    return out;
}
Json to_json(const CodeBlock& code) {
    Array highlighted;
    for (auto line : code.highlighted_lines) highlighted.push_back(line);
    return Object{{"code", js(code.code)}, {"language", js(code.language)}, {"font", to_json(code.font)}, {"show_line_numbers", code.show_line_numbers}, {"highlighted_lines", highlighted}};
}
Json to_json(const Diagram& diagram) { return Object{{"source", js(diagram.source)}, {"language", js(to_string(diagram.language))}}; }

Json to_json(const ComponentSet& components) {
    Object out;
    if (components.transform) out["transform"] = to_json(*components.transform);
    if (components.style) out["style"] = to_json(*components.style);
    if (components.shape) out["shape"] = to_json(*components.shape);
    if (components.text) out["text"] = to_json(*components.text);
    if (components.math) out["math"] = to_json(*components.math);
    if (components.image) out["image"] = to_json(*components.image);
    if (components.code) out["code"] = to_json(*components.code);
    if (components.diagram) out["diagram"] = to_json(*components.diagram);
    return out;
}

Json to_json(const Node& node) {
    Object out{{"id", to_json(node.id)}, {"node_type", js(to_string(node.node_type))}, {"components", to_json(node.components)}};
    if (node.name) out["name"] = js(*node.name);
    if (node.parent) out["parent"] = to_json(*node.parent);
    if (!node.children.empty()) out["children"] = ids_to_json(node.children);
    return out;
}

Json to_json(const Keyframe& keyframe) {
    return Object{{"time", to_json(keyframe.time)}, {"value", property_value_to_json(keyframe.value)}, {"easing", js(to_string(keyframe.easing))}};
}

Json to_json(const KeyframeTrack& track) {
    Array keyframes;
    for (const auto& keyframe : track.keyframes) keyframes.push_back(to_json(keyframe));
    return Object{{"target_node", to_json(track.target_node)}, {"property", js(to_string(track.property))}, {"keyframes", keyframes}};
}

Json to_json(const Timeline& timeline) {
    Object out{{"duration", to_json(timeline.duration)}};
    Array tracks;
    for (const auto& track : timeline.tracks) tracks.push_back(to_json(track));
    if (!tracks.empty()) out["tracks"] = tracks;
    return out;
}

Json to_json(const Scene& scene) {
    Array nodes;
    for (const auto& node : scene.nodes) nodes.push_back(to_json(node));
    return Object{{"id", to_json(scene.id)}, {"name", js(scene.name)}, {"root_node", to_json(scene.root_node)}, {"nodes", nodes}, {"timeline", to_json(scene.timeline)}, {"duration", to_json(scene.duration)}, {"metadata", Object{}}};
}

Json to_json(const RenderSettings& settings) {
    return Object{{"resolution", Array{settings.resolution.first, settings.resolution.second}}, {"fps", settings.fps}, {"format", js(to_string(settings.format))}, {"background_color", to_json(settings.background_color)}, {"anti_aliasing", settings.anti_aliasing}, {"pixel_scale", settings.pixel_scale}};
}

Json to_json(const Project& project) {
    Object metadata{{"name", js(project.metadata.name)}, {"version", js(project.metadata.version)}};
    if (project.metadata.description) metadata["description"] = js(*project.metadata.description);
    Array authors;
    for (const auto& author : project.metadata.authors) authors.push_back(js(author));
    if (!authors.empty()) metadata["authors"] = authors;
    Array scenes;
    for (const auto& scene : project.scenes) scenes.push_back(to_json(scene));
    Array assets;
    for (const auto& asset : project.assets.assets) assets.push_back(to_json(asset));
    return Object{{"id", to_json(project.id)}, {"metadata", metadata}, {"scenes", scenes}, {"assets", Object{{"assets", assets}}}, {"settings", to_json(project.settings)}};
}

Transform transform_from_json(const Json& json) {
    Transform transform;
    if (!json.is_object()) return transform;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "position")) transform.position = vec2_from_json(*value);
    if (const auto* value = find(object, "scale")) transform.scale = vec2_from_json(*value, Vec2::one());
    if (const auto* value = find(object, "anchor")) transform.anchor = vec2_from_json(*value, Vec2::half());
    transform.rotation = number_or(object, "rotation", transform.rotation);
    return transform;
}

Style style_from_json(const Json& json) {
    Style style;
    if (!json.is_object()) return style;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "fill")) style.fill = color_from_json(*value);
    if (const auto* value = find(object, "stroke")) style.stroke = stroke_from_json(*value);
    style.opacity = number_or(object, "opacity", style.opacity);
    style.z_index = static_cast<int>(number_or(object, "z_index", style.z_index));
    style.visible = bool_or(object, "visible", style.visible);
    return style;
}

Shape shape_from_json(const Json& json) {
    Shape shape;
    if (!json.is_object()) return shape;
    const auto* kind_value = find(json.as_object(), "kind");
    if (!kind_value || !kind_value->is_object()) return shape;
    const auto& object = kind_value->as_object();
    auto& k = shape.kind;
    k.type = shape_type_from_string(string_or(object, "type", "circle"));
    k.width = number_or(object, "width");
    k.height = number_or(object, "height");
    k.corner_radius = number_or(object, "corner_radius");
    k.radius = number_or(object, "radius");
    k.rx = number_or(object, "rx");
    k.ry = number_or(object, "ry");
    if (const auto* value = find(object, "start")) k.start = vec2_from_json(*value);
    if (const auto* value = find(object, "end")) k.end = vec2_from_json(*value);
    if (const auto* value = find(object, "points")) k.points = vec2s_from_json(*value);
    k.closed = bool_or(object, "closed", false);
    k.data = string_or(object, "data");
    k.start_angle = number_or(object, "start_angle");
    k.end_angle = number_or(object, "end_angle");
    k.head_size = number_or(object, "head_size", 10.0);
    return shape;
}

TextContent text_from_json(const Json& json) {
    TextContent text;
    if (!json.is_object()) return text;
    const auto& object = json.as_object();
    text.content = string_or(object, "content");
    if (const auto* value = find(object, "font")) text.font = font_from_json(*value);
    text.align = text_align_from_string(string_or(object, "align", "left"));
    text.vertical_align = text_vertical_align_from_string(string_or(object, "vertical_align", "top"));
    if (const auto* value = find(object, "max_width"); value && value->is_number()) text.max_width = value->to_number<double>();
    text.line_height = number_or(object, "line_height", text.line_height);
    return text;
}

MathExpression math_from_json(const Json& json) {
    MathExpression math;
    if (!json.is_object()) return math;
    const auto& object = json.as_object();
    math.latex = string_or(object, "latex");
    math.font_size = number_or(object, "font_size", math.font_size);
    if (const auto* value = find(object, "color")) math.color = color_from_json(*value);
    return math;
}

ImageContent image_from_json(const Json& json) {
    ImageContent image;
    if (!json.is_object()) return image;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "asset_ref")) image.asset_ref = asset_from_json(*value);
    image.fit = image_fit_from_string(string_or(object, "fit", "contain"));
    if (const auto* value = find(object, "width"); value && value->is_number()) image.width = value->to_number<double>();
    if (const auto* value = find(object, "height"); value && value->is_number()) image.height = value->to_number<double>();
    return image;
}

CodeBlock code_from_json(const Json& json) {
    CodeBlock code;
    if (!json.is_object()) return code;
    const auto& object = json.as_object();
    code.code = string_or(object, "code");
    code.language = string_or(object, "language");
    if (const auto* value = find(object, "font")) code.font = font_from_json(*value);
    code.show_line_numbers = bool_or(object, "show_line_numbers", false);
    return code;
}

Diagram diagram_from_json(const Json& json) {
    Diagram diagram;
    if (!json.is_object()) return diagram;
    const auto& object = json.as_object();
    diagram.source = string_or(object, "source");
    diagram.language = diagram_language_from_string(string_or(object, "language", "mermaid"));
    return diagram;
}

ComponentSet components_from_json(const Json& json) {
    ComponentSet components;
    if (!json.is_object()) return components;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "transform")) components.transform = transform_from_json(*value);
    if (const auto* value = find(object, "style")) components.style = style_from_json(*value);
    if (const auto* value = find(object, "shape")) components.shape = shape_from_json(*value);
    if (const auto* value = find(object, "text")) components.text = text_from_json(*value);
    if (const auto* value = find(object, "math")) components.math = math_from_json(*value);
    if (const auto* value = find(object, "image")) components.image = image_from_json(*value);
    if (const auto* value = find(object, "code")) components.code = code_from_json(*value);
    if (const auto* value = find(object, "diagram")) components.diagram = diagram_from_json(*value);
    return components;
}

Node node_from_json(const Json& json) {
    Node node;
    if (!json.is_object()) return node;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "id")) node.id = id_from_json(*value);
    node.name = opt_string(object, "name");
    node.node_type = node_type_from_string(string_or(object, "node_type", "group"));
    if (const auto* value = find(object, "parent")) node.parent = id_from_json(*value);
    if (const auto* value = find(object, "children")) node.children = ids_from_json(*value);
    if (const auto* value = find(object, "components")) node.components = components_from_json(*value);
    return node;
}

Timeline timeline_from_json(const Json& json) {
    Timeline timeline;
    if (!json.is_object()) return timeline;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "duration")) timeline.duration = duration_from_json(*value);
    if (const auto* tracks = find(object, "tracks"); tracks && tracks->is_array()) {
        for (const auto& item : tracks->as_array()) {
            if (!item.is_object()) continue;
            const auto& track_obj = item.as_object();
            KeyframeTrack track;
            if (const auto* value = find(track_obj, "target_node")) track.target_node = id_from_json(*value);
            track.property = animatable_property_from_string(string_or(track_obj, "property", "opacity"));
            if (const auto* keyframes = find(track_obj, "keyframes"); keyframes && keyframes->is_array()) {
                for (const auto& frame_json : keyframes->as_array()) {
                    if (!frame_json.is_object()) continue;
                    const auto& frame_obj = frame_json.as_object();
                    Keyframe frame;
                    if (const auto* value = find(frame_obj, "time")) frame.time = duration_from_json(*value);
                    if (const auto* value = find(frame_obj, "value")) frame.value = property_value_from_json(*value);
                    frame.easing = easing_from_string(string_or(frame_obj, "easing", "ease_in_out"));
                    track.keyframes.push_back(frame);
                }
            }
            timeline.tracks.push_back(track);
        }
    }
    return timeline;
}

Scene scene_from_json(const Json& json) {
    Scene scene;
    if (!json.is_object()) return scene;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "id")) scene.id = id_from_json(*value);
    scene.name = string_or(object, "name");
    if (const auto* value = find(object, "root_node")) scene.root_node = id_from_json(*value);
    if (const auto* nodes = find(object, "nodes"); nodes && nodes->is_array()) {
        for (const auto& node_json : nodes->as_array()) scene.nodes.push_back(node_from_json(node_json));
    }
    if (const auto* value = find(object, "timeline")) scene.timeline = timeline_from_json(*value);
    if (const auto* value = find(object, "duration")) scene.duration = duration_from_json(*value);
    return scene;
}

RenderSettings render_settings_from_json(const Json& json) {
    RenderSettings settings;
    if (!json.is_object()) return settings;
    const auto& object = json.as_object();
    if (const auto* resolution = find(object, "resolution"); resolution && resolution->is_array() && resolution->as_array().size() >= 2) {
        settings.resolution = {static_cast<std::uint32_t>(resolution->as_array()[0].to_number<int>()), static_cast<std::uint32_t>(resolution->as_array()[1].to_number<int>())};
    }
    settings.fps = number_or(object, "fps", settings.fps);
    settings.format = output_format_from_string(string_or(object, "format", "mp4"));
    if (const auto* value = find(object, "background_color")) settings.background_color = color_from_json(*value, settings.background_color);
    settings.anti_aliasing = static_cast<std::uint32_t>(number_or(object, "anti_aliasing", settings.anti_aliasing));
    settings.pixel_scale = number_or(object, "pixel_scale", settings.pixel_scale);
    settings.quality_preset = opt_string(object, "quality_preset");
    return settings;
}

Project project_from_json(const Json& json) {
    Project project;
    if (!json.is_object()) return project;
    const auto& object = json.as_object();
    if (const auto* value = find(object, "id")) project.id = id_from_json(*value);
    if (const auto* metadata = find(object, "metadata"); metadata && metadata->is_object()) {
        const auto& meta = metadata->as_object();
        project.metadata.name = string_or(meta, "name", project.metadata.name);
        project.metadata.description = opt_string(meta, "description");
        project.metadata.version = string_or(meta, "version", project.metadata.version);
        if (const auto* authors = find(meta, "authors")) project.metadata.authors = strings_from_json(*authors);
        project.metadata.created_at = opt_string(meta, "created_at");
        project.metadata.modified_at = opt_string(meta, "modified_at");
        if (const auto* tags = find(meta, "tags")) project.metadata.tags = strings_from_json(*tags);
    }
    if (const auto* scenes = find(object, "scenes"); scenes && scenes->is_array()) {
        for (const auto& scene_json : scenes->as_array()) project.scenes.push_back(scene_from_json(scene_json));
    }
    if (const auto* settings = find(object, "settings")) project.settings = render_settings_from_json(*settings);
    return project;
}

Project load_project_file(const std::filesystem::path& path) {
    std::ifstream file(path);
    if (!file) throw std::runtime_error("failed to open project file: " + path.string());
    std::string source((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    return project_from_json(boost::json::parse(source));
}

void save_project_file(const Project& project, const std::filesystem::path& path) {
    std::ofstream file(path);
    if (!file) throw std::runtime_error("failed to write project file: " + path.string());
    file << boost::json::serialize(to_json(project)) << '\n';
}

}  // namespace openanim
