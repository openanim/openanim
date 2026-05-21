//! OpenAnim CLI — command-line interface for the animation engine.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use scene_ir::project::Project;


#[derive(Parser)]
#[command(name = "openanim", version, about = "OpenAnim Animation Engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a project or scene JSON file.
    Validate {
        /// Path to the .json file.
        file: PathBuf,
    },

    /// Export JSON schemas for IR types.
    Schema {
        /// Export only a specific type's schema.
        #[arg(long)]
        type_name: Option<String>,

        /// Write to file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Print summary info about a project file.
    Info {
        /// Path to the project .json file.
        file: PathBuf,
    },

    /// Diff two project files.
    Diff {
        /// Path to the old .json file.
        old: PathBuf,
        /// Path to the new .json file.
        new: PathBuf,
    },

    /// Compute content hash of a project file.
    Hash {
        /// Path to the .json file.
        file: PathBuf,
    },

    /// Render a project file or specific scene.
    Render {
        /// Path to the project .json file.
        file: PathBuf,

        /// Specific scene index (0-based) or name to render. If omitted, renders all scenes.
        #[arg(long)]
        scene: Option<String>,

        /// Path to directory to use for caching.
        #[arg(long, default_value = ".openanim/cache")]
        cache_dir: PathBuf,
    },

    /// List all registered renderer adapters and their status.
    ListRenderers,

    /// Launch the interactive local web Studio dashboard
    Studio {
        /// Local port to run the web server on
        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Path to the workspace directory to serve
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Schema { type_name, output } => cmd_schema(type_name.as_deref(), output.as_deref()),
        Commands::Info { file } => cmd_info(&file),
        Commands::Diff { old, new } => cmd_diff(&old, &new),
        Commands::Hash { file } => cmd_hash(&file),
        Commands::Render { file, scene, cache_dir } => cmd_render(&file, scene.as_deref(), &cache_dir).await,
        Commands::ListRenderers => cmd_list_renderers(),
        Commands::Studio { port, dir } => cmd_studio(port, dir).await,
    }
}

fn load_project(path: &std::path::Path) -> Result<Project> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    let project: Project = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse project JSON: {}", path.display()))?;
    Ok(project)
}

fn cmd_validate(file: &std::path::Path) -> Result<()> {
    let project = load_project(file)?;
    let errors = scene_ir::validation::validate_project(&project);

    if errors.is_empty() {
        println!("✅ Valid! No errors found.");
        println!("   Project: {}", project.metadata.name);
        println!("   Scenes:  {}", project.scene_count());
        println!("   Nodes:   {}", project.total_node_count());
    } else {
        println!("❌ Found {} validation error(s):", errors.len());
        for (i, err) in errors.iter().enumerate() {
            println!("   {}. {}", i + 1, err);
        }
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_schema(type_name: Option<&str>, output: Option<&std::path::Path>) -> Result<()> {
    let json = match type_name {
        Some(name) => {
            let schemas = scene_ir::schema::all_schemas();
            match schemas.get(name) {
                Some(schema) => serde_json::to_string_pretty(schema)?,
                None => {
                    let available: Vec<&String> = schemas.keys().collect();
                    anyhow::bail!(
                        "Unknown type '{}'. Available types: {:?}",
                        name,
                        available
                    );
                }
            }
        }
        None => {
            let schemas = scene_ir::schema::all_schemas();
            serde_json::to_string_pretty(&schemas)?
        }
    };

    match output {
        Some(path) => {
            std::fs::write(path, &json)
                .with_context(|| format!("Failed to write: {}", path.display()))?;
            println!("Schema written to {}", path.display());
        }
        None => {
            println!("{json}");
        }
    }

    Ok(())
}

fn cmd_info(file: &std::path::Path) -> Result<()> {
    let project = load_project(file)?;

    println!("📋 Project Info");
    println!("   Name:       {}", project.metadata.name);
    println!("   Version:    {}", project.metadata.version);
    if let Some(desc) = &project.metadata.description {
        println!("   Description: {desc}");
    }
    println!("   Scenes:     {}", project.scene_count());
    println!("   Total nodes: {}", project.total_node_count());
    println!(
        "   Resolution: {}x{}",
        project.settings.resolution.0, project.settings.resolution.1
    );
    println!("   FPS:        {}", project.settings.fps);
    println!("   Format:     {:?}", project.settings.format);

    for (i, scene) in project.scenes.iter().enumerate() {
        println!("   Scene {}: \"{}\" ({} nodes)", i + 1, scene.name, scene.node_count());
    }

    Ok(())
}

fn cmd_diff(old_path: &std::path::Path, new_path: &std::path::Path) -> Result<()> {
    let old_project = load_project(old_path)?;
    let new_project = load_project(new_path)?;

    let min_scenes = old_project.scenes.len().min(new_project.scenes.len());

    for i in 0..min_scenes {
        let diff = scene_diff::diff::diff_scenes(&old_project.scenes[i], &new_project.scenes[i]);
        if diff.is_empty() {
            println!("Scene {}: no changes", i);
        } else {
            println!("Scene {}: {} change(s)", i, diff.op_count());
            let json = serde_json::to_string_pretty(&diff)?;
            println!("{json}");
        }
    }

    if new_project.scenes.len() > old_project.scenes.len() {
        println!(
            "{} new scene(s) added",
            new_project.scenes.len() - old_project.scenes.len()
        );
    } else if old_project.scenes.len() > new_project.scenes.len() {
        println!(
            "{} scene(s) removed",
            old_project.scenes.len() - new_project.scenes.len()
        );
    }

    Ok(())
}

fn cmd_hash(file: &std::path::Path) -> Result<()> {
    let project = load_project(file)?;
    let hash = hasher::hash_project(&project);
    println!("🔑 Content hash: {hash}");

    for (i, scene) in project.scenes.iter().enumerate() {
        let scene_hash = hasher::hash_scene(scene);
        println!("   Scene {i} (\"{}\"): {scene_hash}", scene.name);
    }

    Ok(())
}

async fn cmd_render(
    file: &std::path::Path,
    scene_filter: Option<&str>,
    cache_dir: &std::path::Path,
) -> Result<()> {
    let project = load_project(file)?;
    let orchestrator = orchestrator::Orchestrator::new(cache_dir.to_path_buf());

    let scenes_to_render = if let Some(filter) = scene_filter {
        if let Ok(idx) = filter.parse::<usize>() {
            if idx < project.scenes.len() {
                vec![&project.scenes[idx]]
            } else {
                anyhow::bail!(
                    "Scene index {} is out of bounds (project has {} scenes)",
                    idx,
                    project.scenes.len()
                );
            }
        } else {
            let found: Vec<&scene_ir::scene::Scene> = project
                .scenes
                .iter()
                .filter(|s| s.name.eq_ignore_ascii_case(filter))
                .collect();
            if found.is_empty() {
                anyhow::bail!("No scene found with name '{}'", filter);
            }
            found
        }
    } else {
        project.scenes.iter().collect()
    };

    println!(
        "🚀 Rendering {} scene(s) from project \"{}\"...",
        scenes_to_render.len(),
        project.metadata.name
    );

    for (i, scene) in scenes_to_render.iter().enumerate() {
        println!("🎬 Scene [{}]: \"{}\" ({} nodes)", i + 1, scene.name, scene.node_count());
        let start = std::time::Instant::now();
        let artifact = orchestrator.render(scene, &project.settings).await?;
        let duration = start.elapsed();

        println!("   Status:      {:?}", artifact.status);
        println!("   Output path: {}", artifact.output_path.display());
        println!("   Content hash: {:?}", artifact.content_hash.as_deref().unwrap_or("none"));
        if !artifact.stdout.is_empty() {
            println!("   Stdout:      {}", artifact.stdout.trim());
        }
        if !artifact.stderr.is_empty() {
            println!("   Stderr:      {}", artifact.stderr.trim());
        }
        println!("   Duration:    {:.2?}", duration);
    }

    println!("✨ Render pipeline execution finished!");
    Ok(())
}

fn cmd_list_renderers() -> Result<()> {
    let orchestrator = orchestrator::Orchestrator::new(std::env::temp_dir());
    println!("Available Renderer Adapters:");
    let list = orchestrator.registry.list();
    if list.is_empty() {
        println!("  (No renderer adapters registered)");
    } else {
        for name in list {
            if let Some(adapter) = orchestrator.registry.get(name) {
                println!("  - {:<10} (version: {})", name, adapter.version());
                println!("    supported nodes: {:?}", adapter.supported_node_types());
            }
        }
    }
    Ok(())
}

async fn cmd_studio(port: u16, dir: Option<PathBuf>) -> Result<()> {
    let workspace_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let workspace_dir = std::fs::canonicalize(&workspace_dir)
        .context("Failed to canonicalize workspace directory")?;
    
    // Spawn browser in a separate thread or tokio task to prevent blocking
    let url = format!("http://localhost:{}", port);
    let browser_url = url.clone();
    tokio::task::spawn_blocking(move || {
        // Wait a short moment for the server to bind
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!("🔗 Opening OpenAnim Studio in your default browser: {}", browser_url);
        if let Err(e) = webbrowser::open(&browser_url) {
            eprintln!("⚠️ Failed to automatically open web browser: {}", e);
        }
    });

    studio::start_server(port, workspace_dir).await?;
    Ok(())
}
