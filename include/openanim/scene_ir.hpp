#pragma once

#include "openanim/types.hpp"

#include <filesystem>
#include <map>
#include <memory>
#include <variant>

namespace openanim {

enum class NodeType { Group, Shape, Text, Math, Image, Code, Diagram, Custom };
enum class ShapeType { Rectangle, Circle, Ellipse, Line, Polygon, Path, Arc, Arrow };
enum class DiagramLanguage { Mermaid, PlantUml, Graphviz, D2 };
enum class Alignment { TopLeft, TopCenter, TopRight, CenterLeft, Center, CenterRight, BottomLeft, BottomCenter, BottomRight };
enum class Direction { Up, Down, Left, Right };
enum class AnimatableProperty { PositionX, PositionY, Position, Rotation, ScaleX, ScaleY, Scale, Opacity, FillColor, StrokeColor, StrokeWidth };
enum class EasingFunction { Linear, EaseIn, EaseOut, EaseInOut, EaseInQuad, EaseOutQuad, EaseInOutQuad, Step };
enum class EventType { FadeIn, FadeOut, Create, Write, Transform, MoveTo, Rotate, ScaleTo, Custom };

struct Transform {
    Vec2 position = Vec2::zero();
    double rotation = 0.0;
    Vec2 scale = Vec2::one();
    Vec2 anchor = Vec2::half();

    static Transform at(double x, double y);
};

struct Style {
    std::optional<Color> fill;
    std::optional<Stroke> stroke;
    double opacity = 1.0;
    int z_index = 0;
    bool visible = true;

    static Style with_fill(Color color);
    static Style with_stroke(Stroke stroke);
};

struct ShapeKind {
    ShapeType type = ShapeType::Circle;
    double width = 0.0;
    double height = 0.0;
    double corner_radius = 0.0;
    double radius = 0.0;
    double rx = 0.0;
    double ry = 0.0;
    Vec2 start;
    Vec2 end;
    std::vector<Vec2> points;
    bool closed = false;
    std::string data;
    double start_angle = 0.0;
    double end_angle = 0.0;
    double head_size = 10.0;
};

struct Shape {
    ShapeKind kind;
};

struct TextContent {
    std::string content;
    FontSpec font;
    TextAlign align = TextAlign::Left;
    TextVerticalAlign vertical_align = TextVerticalAlign::Top;
    std::optional<double> max_width;
    double line_height = 1.4;
};

struct MathExpression {
    std::string latex;
    double font_size = 36.0;
    Color color = Color::white();
};

struct ImageContent {
    AssetRef asset_ref;
    ImageFit fit = ImageFit::Contain;
    std::optional<double> width;
    std::optional<double> height;
};

struct CodeBlock {
    std::string code;
    std::string language;
    FontSpec font = {"JetBrains Mono", 18.0, FontWeight::Regular, FontStyle::Normal};
    bool show_line_numbers = false;
    std::vector<std::uint32_t> highlighted_lines;
};

struct Diagram {
    std::string source;
    DiagramLanguage language = DiagramLanguage::Mermaid;
};

struct LayoutConstraint {
    std::string type;
    std::optional<NodeId> target;
    std::optional<Alignment> alignment;
    Vec2 offset;
    std::optional<double> distance;
    std::optional<Direction> direction;
    bool horizontal = true;
    bool vertical = true;
};

struct ComponentSet {
    std::optional<Transform> transform;
    std::optional<Style> style;
    std::optional<Shape> shape;
    std::optional<TextContent> text;
    std::optional<MathExpression> math;
    std::optional<ImageContent> image;
    std::optional<CodeBlock> code;
    std::optional<Diagram> diagram;
    std::vector<LayoutConstraint> constraints;
};

struct Node {
    NodeId id = NodeId::make();
    std::optional<std::string> name;
    NodeType node_type = NodeType::Group;
    std::optional<NodeId> parent;
    std::vector<NodeId> children;
    ComponentSet components;

    static Node make(NodeType type);
    static Node named(std::string name, NodeType type);
    bool is_renderable() const;
    bool is_leaf() const;
    bool is_root() const;
};

using PropertyValue = std::variant<double, Vec2, Color>;

struct Keyframe {
    DurationSecs time;
    PropertyValue value = 0.0;
    EasingFunction easing = EasingFunction::EaseInOut;
};

struct KeyframeTrack {
    NodeId target_node;
    AnimatableProperty property = AnimatableProperty::Opacity;
    std::vector<Keyframe> keyframes;
};

struct EventParams {
    std::optional<Vec2> target_position;
    std::optional<double> target_rotation;
    std::optional<Vec2> target_scale;
    std::optional<NodeId> target_node;
    std::vector<Vec2> path_points;
    EasingFunction easing = EasingFunction::EaseInOut;
    std::vector<EventId> group_events;
    std::vector<EventId> sequence_events;
    std::map<std::string, Json> custom;
};

struct AnimationEvent {
    EventId id = EventId::make();
    DurationSecs start_time;
    DurationSecs duration;
    EventType event_type = EventType::Custom;
    std::vector<NodeId> target_nodes;
    EventParams params;
    std::optional<std::string> label;
};

struct Timeline {
    DurationSecs duration;
    std::vector<KeyframeTrack> tracks;
    std::vector<AnimationEvent> events;

    std::vector<const AnimationEvent*> sorted_events() const;
};

struct SceneRenderOverride {
    std::optional<std::pair<std::uint32_t, std::uint32_t>> resolution;
    std::optional<double> fps;
};

struct SceneMetadata {
    std::optional<std::string> description;
    std::vector<std::string> tags;
    std::optional<Color> background_color;
    std::optional<SceneRenderOverride> render_override;
};

struct Scene {
    SceneId id = SceneId::make();
    std::string name;
    NodeId root_node;
    std::vector<Node> nodes;
    Timeline timeline;
    DurationSecs duration;
    SceneMetadata metadata;

    static Scene make(std::string name);
    const Node* find_node(const NodeId& id) const;
    Node* find_node(const NodeId& id);
    const Node* find_node_by_name(std::string_view name) const;
    std::size_t node_count() const;
};

struct RenderSettings {
    std::pair<std::uint32_t, std::uint32_t> resolution = {1920, 1080};
    double fps = 30.0;
    OutputFormat format = OutputFormat::Mp4;
    Color background_color = Color::black();
    std::uint32_t anti_aliasing = 4;
    double pixel_scale = 1.0;
    std::optional<std::string> quality_preset;
};

struct ProjectMetadata {
    std::string name = "Untitled Project";
    std::optional<std::string> description;
    std::string version = "0.1.0";
    std::vector<std::string> authors;
    std::optional<std::string> created_at;
    std::optional<std::string> modified_at;
    std::vector<std::string> tags;
};

struct AssetManifest {
    std::vector<AssetRef> assets;

    const AssetRef* find_asset(std::string_view asset_id) const;
    void add_asset(AssetRef asset);
};

struct Project {
    ProjectId id = ProjectId::make();
    ProjectMetadata metadata;
    std::vector<Scene> scenes;
    AssetManifest assets;
    RenderSettings settings;

    static Project make(std::string name);
    void add_scene(Scene scene);
    std::size_t scene_count() const;
    std::size_t total_node_count() const;
};

std::string to_string(NodeType value);
std::string to_string(ShapeType value);
std::string to_string(DiagramLanguage value);
std::string to_string(AnimatableProperty value);
std::string to_string(EasingFunction value);
std::string to_string(EventType value);

NodeType node_type_from_string(std::string_view value);
ShapeType shape_type_from_string(std::string_view value);
DiagramLanguage diagram_language_from_string(std::string_view value);
AnimatableProperty animatable_property_from_string(std::string_view value);
EasingFunction easing_from_string(std::string_view value);
EventType event_type_from_string(std::string_view value);

Json to_json(const Transform& transform);
Json to_json(const Style& style);
Json to_json(const Shape& shape);
Json to_json(const TextContent& text);
Json to_json(const MathExpression& math);
Json to_json(const ImageContent& image);
Json to_json(const CodeBlock& code);
Json to_json(const Diagram& diagram);
Json to_json(const ComponentSet& components);
Json to_json(const Node& node);
Json to_json(const Keyframe& keyframe);
Json to_json(const KeyframeTrack& track);
Json to_json(const Timeline& timeline);
Json to_json(const Scene& scene);
Json to_json(const RenderSettings& settings);
Json to_json(const Project& project);

Transform transform_from_json(const Json& json);
Style style_from_json(const Json& json);
Shape shape_from_json(const Json& json);
TextContent text_from_json(const Json& json);
MathExpression math_from_json(const Json& json);
ImageContent image_from_json(const Json& json);
CodeBlock code_from_json(const Json& json);
Diagram diagram_from_json(const Json& json);
ComponentSet components_from_json(const Json& json);
Node node_from_json(const Json& json);
Timeline timeline_from_json(const Json& json);
Scene scene_from_json(const Json& json);
RenderSettings render_settings_from_json(const Json& json);
Project project_from_json(const Json& json);

Project load_project_file(const std::filesystem::path& path);
void save_project_file(const Project& project, const std::filesystem::path& path);

}  // namespace openanim
