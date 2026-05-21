//! Top-level project definition, render settings, and asset manifest.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::scene::Scene;
use crate::types::{AssetRef, Color, OutputFormat, ProjectId};

/// The top-level OpenAnim project.
///
/// A project contains one or more scenes, global render settings,
/// and an asset manifest. This is the root of the IR hierarchy:
///
/// ```text
/// Project
/// ├── ProjectMetadata
/// ├── RenderSettings
/// ├── AssetManifest
/// └── Vec<Scene>
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    /// Unique project identifier.
    pub id: ProjectId,

    /// Project metadata.
    pub metadata: ProjectMetadata,

    /// Ordered list of scenes in this project.
    pub scenes: Vec<Scene>,

    /// External asset references.
    #[serde(default)]
    pub assets: AssetManifest,

    /// Global render settings.
    #[serde(default)]
    pub settings: RenderSettings,
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::new(),
            metadata: ProjectMetadata {
                name: name.into(),
                ..Default::default()
            },
            scenes: Vec::new(),
            assets: AssetManifest::default(),
            settings: RenderSettings::default(),
        }
    }

    /// Add a scene to the project.
    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }

    /// Get the total number of scenes.
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// Get the total number of nodes across all scenes.
    pub fn total_node_count(&self) -> usize {
        self.scenes.iter().map(|s| s.node_count()).sum()
    }
}

/// Project metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectMetadata {
    /// Project name.
    pub name: String,

    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Project version (semver).
    #[serde(default = "default_version")]
    pub version: String,

    /// Authors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last modification timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    /// Tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            description: None,
            version: default_version(),
            authors: Vec::new(),
            created_at: None,
            modified_at: None,
            tags: Vec::new(),
        }
    }
}

/// Global render settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenderSettings {
    /// Output resolution (width, height) in pixels.
    #[serde(default = "default_resolution")]
    pub resolution: (u32, u32),

    /// Frames per second.
    #[serde(default = "default_fps")]
    pub fps: f64,

    /// Output format.
    #[serde(default)]
    pub format: OutputFormat,

    /// Default background color.
    #[serde(default = "default_background")]
    pub background_color: Color,

    /// Anti-aliasing samples (1 = none, 4 = 4x MSAA, etc.).
    #[serde(default = "default_aa")]
    pub anti_aliasing: u32,

    /// Pixel scale factor for high-DPI rendering.
    #[serde(default = "default_pixel_scale")]
    pub pixel_scale: f64,

    /// Quality preset name (for renderer-specific tuning).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_preset: Option<String>,
}

fn default_resolution() -> (u32, u32) {
    (1920, 1080)
}

fn default_fps() -> f64 {
    30.0
}

fn default_background() -> Color {
    Color::BLACK
}

fn default_aa() -> u32 {
    4
}

fn default_pixel_scale() -> f64 {
    1.0
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            resolution: default_resolution(),
            fps: default_fps(),
            format: OutputFormat::default(),
            background_color: default_background(),
            anti_aliasing: default_aa(),
            pixel_scale: default_pixel_scale(),
            quality_preset: None,
        }
    }
}

/// Manifest of external assets referenced by the project.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AssetManifest {
    /// All assets referenced by this project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<AssetRef>,
}

impl AssetManifest {
    /// Find an asset by its ID.
    pub fn find_asset(&self, asset_id: &str) -> Option<&AssetRef> {
        self.assets.iter().find(|a| a.asset_id == asset_id)
    }

    /// Add an asset to the manifest.
    pub fn add_asset(&mut self, asset: AssetRef) {
        self.assets.push(asset);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    #[test]
    fn test_project_new() {
        let project = Project::new("My Animation");
        assert_eq!(project.metadata.name, "My Animation");
        assert_eq!(project.scene_count(), 0);
        assert_eq!(project.total_node_count(), 0);
    }

    #[test]
    fn test_project_add_scene() {
        let mut project = Project::new("Test");
        project.add_scene(Scene::new("Scene 1"));
        project.add_scene(Scene::new("Scene 2"));
        assert_eq!(project.scene_count(), 2);
        assert_eq!(project.total_node_count(), 2); // 2 root nodes
    }

    #[test]
    fn test_render_settings_default() {
        let rs = RenderSettings::default();
        assert_eq!(rs.resolution, (1920, 1080));
        assert_eq!(rs.fps, 30.0);
        assert_eq!(rs.background_color, Color::BLACK);
    }

    #[test]
    fn test_project_serde_roundtrip() {
        let mut project = Project::new("Roundtrip Test");
        project.add_scene(Scene::new("Intro"));
        project.metadata.description = Some("A test project".to_string());

        let json = serde_json::to_string_pretty(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, deserialized);
    }

    #[test]
    fn test_asset_manifest() {
        let mut manifest = AssetManifest::default();
        manifest.add_asset(AssetRef {
            asset_id: "logo".to_string(),
            path: "assets/logo.png".to_string(),
            mime_type: Some("image/png".to_string()),
            content_hash: None,
        });

        assert!(manifest.find_asset("logo").is_some());
        assert!(manifest.find_asset("missing").is_none());
    }
}
