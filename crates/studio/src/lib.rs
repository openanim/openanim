use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use orchestrator::runner::Orchestrator;
use scene_ir::project::Project;
use scene_ir::validation::validate_scene;
use llm_compiler::{LlmCompiler, LlmProvider};

#[derive(Clone)]
pub struct AppState {
    pub workspace_dir: PathBuf,
    pub cache_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub engine_version: String,
    pub adapters: AdaptersStatus,
}

#[derive(Serialize, Deserialize)]
pub struct AdaptersStatus {
    pub ffmpeg: bool,
    pub manim: bool,
    pub mermaid: bool,
    pub remotion: bool,
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub provider: LlmProvider,
}

#[derive(Deserialize)]
pub struct PatchRequest {
    pub prompt: String,
    pub provider: LlmProvider,
    pub scene_id: String,
}

#[derive(Deserialize)]
pub struct RenderRequest {
    pub scene_id: String,
}

#[derive(Serialize)]
pub struct RenderResponse {
    pub status: String,
    pub video_hash: String,
    pub output_path: String,
}

/// Start the Axum web server on the specified port.
pub async fn start_server(port: u16, workspace_dir: PathBuf) -> anyhow::Result<()> {
    let cache_dir = workspace_dir.join(".cli_cache");
    fs::create_dir_all(&cache_dir)?;

    let state = Arc::new(AppState {
        workspace_dir,
        cache_dir,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/status", get(handle_status))
        .route("/api/project", get(handle_get_project).post(handle_save_project))
        .route("/api/generate", post(handle_generate_scene))
        .route("/api/patch", post(handle_patch_scene))
        .route("/api/render", post(handle_render_scene))
        .route("/api/video/:hash", get(handle_stream_video))
        .fallback(handle_static_assets)
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    println!("--------------------------------------------------");
    println!("  OpenAnim Studio active at http://localhost:{}", port);
    println!("  Local-first engine running in 100% Rust!");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_status() -> Json<StatusResponse> {
    let ffmpeg = Command::new("ffmpeg").arg("-version").output().is_ok();
    let manim = Command::new("manim").arg("--version").output().is_ok();
    let mermaid = Command::new("mmdc").arg("--version").output().is_ok();
    let remotion = Command::new("npx").arg("remotion").arg("--version").output().is_ok();

    Json(StatusResponse {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        adapters: AdaptersStatus {
            ffmpeg,
            manim,
            mermaid,
            remotion,
        },
    })
}

async fn handle_get_project(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let project_path = state.workspace_dir.join("project.json");
    if !project_path.exists() {
        return (StatusCode::NOT_FOUND, "project.json not found in workspace").into_response();
    }

    match fs::read_to_string(&project_path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse project.json: {}", e)).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read project.json: {}", e)).into_response(),
    }
}

async fn handle_save_project(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let project_path = state.workspace_dir.join("project.json");
    
    // Validate project schema structurally by converting to Project struct
    match serde_json::from_value::<Project>(payload.clone()) {
        Ok(project) => {
            // Also validate scenes
            for scene in &project.scenes {
                let errs = validate_scene(scene);
                if !errs.is_empty() {
                    let err_msg = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                    return (StatusCode::BAD_REQUEST, format!("Invalid scene graph hierarchy: {}", err_msg)).into_response();
                }
            }
        }
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid project JSON schema: {}", e)).into_response();
        }
    }

    match serde_json::to_string_pretty(&payload) {
        Ok(pretty) => match fs::write(&project_path, pretty) {
            Ok(_) => (StatusCode::OK, "Saved project successfully").into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write project.json: {}", e)).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize project: {}", e)).into_response(),
    }
}

async fn handle_generate_scene(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateRequest>,
) -> impl IntoResponse {
    let project_path = state.workspace_dir.join("project.json");
    if !project_path.exists() {
        return (StatusCode::NOT_FOUND, "project.json not found in workspace").into_response();
    }

    let project_content = match fs::read_to_string(&project_path) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read project: {}", e)).into_response(),
    };

    let mut project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse project: {}", e)).into_response(),
    };

    let compiler = LlmCompiler::new(req.provider);
    match compiler.compile_scene(&req.prompt).await {
        Ok(new_scene) => {
            // Check for valid graph hierarchy
            let errs = validate_scene(&new_scene);
            if !errs.is_empty() {
                let err_msg = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                return (StatusCode::BAD_REQUEST, format!("Generated scene graph validation failed: {}", err_msg)).into_response();
            }

            // Append scene to local project and save
            project.scenes.push(new_scene.clone());
            match serde_json::to_string_pretty(&project) {
                Ok(pretty) => {
                    if let Err(e) = fs::write(&project_path, pretty) {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save project.json: {}", e)).into_response();
                    }
                }
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize project: {}", e)).into_response(),
            }

            Json(new_scene).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("LLM compilation error: {}", e)).into_response(),
    }
}

async fn handle_patch_scene(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PatchRequest>,
) -> impl IntoResponse {
    let project_path = state.workspace_dir.join("project.json");
    if !project_path.exists() {
        return (StatusCode::NOT_FOUND, "project.json not found in workspace").into_response();
    }

    let project_content = match fs::read_to_string(&project_path) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read project: {}", e)).into_response(),
    };

    let mut project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse project: {}", e)).into_response(),
    };

    let scene_idx = match project.scenes.iter().position(|s| s.id.to_string() == req.scene_id) {
        Some(idx) => idx,
        None => return (StatusCode::NOT_FOUND, format!("Scene with ID {} not found", req.scene_id)).into_response(),
    };

    let compiler = LlmCompiler::new(req.provider);
    match compiler.patch_scene(&project.scenes[scene_idx], &req.prompt).await {
        Ok(patched_scene) => {
            let errs = validate_scene(&patched_scene);
            if !errs.is_empty() {
                let err_msg = errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                return (StatusCode::BAD_REQUEST, format!("Patched scene validation failed: {}", err_msg)).into_response();
            }

            project.scenes[scene_idx] = patched_scene.clone();
            match serde_json::to_string_pretty(&project) {
                Ok(pretty) => {
                    if let Err(e) = fs::write(&project_path, pretty) {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save project.json: {}", e)).into_response();
                    }
                }
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize project: {}", e)).into_response(),
            }

            Json(patched_scene).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("LLM scene patch compilation error: {}", e)).into_response(),
    }
}

async fn handle_render_scene(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RenderRequest>,
) -> impl IntoResponse {
    let project_path = state.workspace_dir.join("project.json");
    if !project_path.exists() {
        return (StatusCode::NOT_FOUND, "project.json not found in workspace").into_response();
    }

    let project_content = match fs::read_to_string(&project_path) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read project: {}", e)).into_response(),
    };

    let project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse project: {}", e)).into_response(),
    };

    // Find requested scene
    let scene = match project.scenes.iter().find(|s| s.id.to_string() == req.scene_id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, format!("Scene with ID {} not found", req.scene_id)).into_response(),
    };

    let orchestrator = Orchestrator::new(state.cache_dir.clone());
    match orchestrator.render(scene, &project.settings).await {
        Ok(artifact) => {
            Json(RenderResponse {
                status: "success".to_string(),
                video_hash: artifact.content_hash.clone().unwrap_or_default(),
                output_path: artifact.output_path.to_string_lossy().to_string(),
            }).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Orchestrator render execution failed: {}", e)).into_response(),
    }
}

async fn handle_stream_video(
    State(state): State<Arc<AppState>>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    let file_path = state.cache_dir.join(format!("{}.mp4", hash));
    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "Rendered video file not found in cache").into_response();
    }

    match fs::read(&file_path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "video/mp4")
            .body(bytes.into())
            .unwrap(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read video file: {}", e)).into_response(),
    }
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../../studio/dist/"]
struct Assets;

fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

async fn handle_static_assets(uri: axum::http::Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(file) => {
            let content_type = get_content_type(&path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(file.data))
                .unwrap()
        }
        None => {
            // Serve index.html as fallback for client-side routing
            match Assets::get("index.html") {
                Some(file) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(axum::body::Body::from(file.data))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(axum::body::Body::from("404 Not Found"))
                    .unwrap(),
            }
        }
    }
}
