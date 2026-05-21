use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use engine_api::{LlmProvider, OpenAnimEngine};
use scene_ir::project::Project;
use scene_ir::validation::validate_scene;

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
    let cache_dir = workspace_dir.join(".openanim/cache");
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
        .route(
            "/api/project",
            get(handle_get_project).post(handle_save_project),
        )
        .route("/api/generate", post(handle_generate_scene))
        .route("/api/patch", post(handle_patch_scene))
        .route("/api/render", post(handle_render_scene))
        .route("/api/video/:hash", get(handle_stream_video))
        .route("/", get(handle_index))
        .fallback(handle_index)
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
    let remotion = Command::new("npx")
        .arg("remotion")
        .arg("--version")
        .output()
        .is_ok();

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
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse project.json: {}", e),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read project.json: {}", e),
        )
            .into_response(),
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
                    let err_msg = errs
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid scene graph hierarchy: {}", err_msg),
                    )
                        .into_response();
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid project JSON schema: {}", e),
            )
                .into_response();
        }
    }

    match serde_json::to_string_pretty(&payload) {
        Ok(pretty) => match fs::write(&project_path, pretty) {
            Ok(_) => (StatusCode::OK, "Saved project successfully").into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to write project.json: {}", e),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize project: {}", e),
        )
            .into_response(),
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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read project: {}", e),
            )
                .into_response();
        }
    };

    let mut project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse project: {}", e),
            )
                .into_response();
        }
    };

    let engine = OpenAnimEngine::new(state.cache_dir.clone());
    match engine.compile_scene(&req.prompt, req.provider).await {
        Ok(new_scene) => {
            // Check for valid graph hierarchy
            let errs = validate_scene(&new_scene);
            if !errs.is_empty() {
                let err_msg = errs
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Generated scene graph validation failed: {}", err_msg),
                )
                    .into_response();
            }

            // Append scene to local project and save
            project.scenes.push(new_scene.clone());
            match serde_json::to_string_pretty(&project) {
                Ok(pretty) => {
                    if let Err(e) = fs::write(&project_path, pretty) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to save project.json: {}", e),
                        )
                            .into_response();
                    }
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to serialize project: {}", e),
                    )
                        .into_response();
                }
            }

            Json(new_scene).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("LLM compilation error: {}", e),
        )
            .into_response(),
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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read project: {}", e),
            )
                .into_response();
        }
    };

    let mut project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse project: {}", e),
            )
                .into_response();
        }
    };

    let scene_idx = match project
        .scenes
        .iter()
        .position(|s| s.id.to_string() == req.scene_id)
    {
        Some(idx) => idx,
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("Scene with ID {} not found", req.scene_id),
            )
                .into_response();
        }
    };

    let engine = OpenAnimEngine::new(state.cache_dir.clone());
    match engine
        .patch_scene(&project.scenes[scene_idx], &req.prompt, req.provider)
        .await
    {
        Ok(patched_scene) => {
            let errs = validate_scene(&patched_scene);
            if !errs.is_empty() {
                let err_msg = errs
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Patched scene validation failed: {}", err_msg),
                )
                    .into_response();
            }

            project.scenes[scene_idx] = patched_scene.clone();
            match serde_json::to_string_pretty(&project) {
                Ok(pretty) => {
                    if let Err(e) = fs::write(&project_path, pretty) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to save project.json: {}", e),
                        )
                            .into_response();
                    }
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to serialize project: {}", e),
                    )
                        .into_response();
                }
            }

            Json(patched_scene).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("LLM scene patch compilation error: {}", e),
        )
            .into_response(),
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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read project: {}", e),
            )
                .into_response();
        }
    };

    let project: Project = match serde_json::from_str(&project_content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse project: {}", e),
            )
                .into_response();
        }
    };

    // Find requested scene
    let scene = match project
        .scenes
        .iter()
        .find(|s| s.id.to_string() == req.scene_id)
    {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("Scene with ID {} not found", req.scene_id),
            )
                .into_response();
        }
    };

    let engine = OpenAnimEngine::new(state.cache_dir.clone());
    match engine.render_scene(scene, &project.settings).await {
        Ok(artifact) => Json(RenderResponse {
            status: "success".to_string(),
            video_hash: artifact.content_hash.clone().unwrap_or_default(),
            output_path: artifact.output_path.to_string_lossy().to_string(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Engine render execution failed: {}", e),
        )
            .into_response(),
    }
}

async fn handle_stream_video(
    State(state): State<Arc<AppState>>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    let file_path = state.cache_dir.join(format!("{}.mp4", hash));
    if !file_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            "Rendered video file not found in cache",
        )
            .into_response();
    }

    match fs::read(&file_path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "video/mp4")
            .body(bytes.into())
            .unwrap(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read video file: {}", e),
        )
            .into_response(),
    }
}

async fn handle_index() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(axum::body::Body::from(INDEX_HTML))
        .unwrap()
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>OpenAnim Dev Console</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #101114;
      --panel: #181a20;
      --panel-2: #20232b;
      --text: #f1f3f5;
      --muted: #a7adb8;
      --border: #303541;
      --accent: #48b7a0;
      --danger: #ff6b6b;
      --ok: #74c69d;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-height: 100vh;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.45 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header {
      min-height: 56px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 12px 18px;
      border-bottom: 1px solid var(--border);
      background: #14161b;
    }
    h1 { margin: 0; font-size: 16px; font-weight: 650; letter-spacing: 0; }
    main {
      display: grid;
      grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
      min-height: calc(100vh - 56px);
    }
    aside, section { padding: 18px; }
    aside { border-right: 1px solid var(--border); background: var(--panel); }
    .toolbar, .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
    .stack { display: grid; gap: 12px; }
    .box {
      border: 1px solid var(--border);
      background: var(--panel-2);
      border-radius: 6px;
      padding: 12px;
    }
    label { display: grid; gap: 6px; color: var(--muted); font-size: 12px; }
    input, select, textarea, button {
      font: inherit;
      border-radius: 5px;
      border: 1px solid var(--border);
    }
    input, select, textarea {
      width: 100%;
      color: var(--text);
      background: #111318;
      padding: 9px 10px;
    }
    textarea {
      min-height: 140px;
      resize: vertical;
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 12px;
    }
    button {
      color: var(--text);
      background: #242832;
      padding: 8px 11px;
      cursor: pointer;
    }
    button.primary { background: var(--accent); border-color: var(--accent); color: #071210; font-weight: 650; }
    button:disabled { opacity: .55; cursor: wait; }
    .status { color: var(--muted); font-size: 12px; }
    .pill {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      padding: 3px 7px;
      border: 1px solid var(--border);
      border-radius: 999px;
      color: var(--muted);
      font-size: 12px;
    }
    .pill.ok { color: var(--ok); border-color: #315f46; }
    .pill.bad { color: var(--danger); border-color: #744040; }
    #project {
      min-height: calc(100vh - 206px);
      white-space: pre;
      overflow: auto;
    }
    video {
      width: 100%;
      max-height: 420px;
      background: #050505;
      border: 1px solid var(--border);
      border-radius: 6px;
    }
    pre {
      margin: 0;
      max-height: 220px;
      overflow: auto;
      color: var(--muted);
      white-space: pre-wrap;
      overflow-wrap: anywhere;
    }
    @media (max-width: 860px) {
      main { grid-template-columns: 1fr; }
      aside { border-right: 0; border-bottom: 1px solid var(--border); }
      #project { min-height: 360px; }
    }
  </style>
</head>
<body>
  <header>
    <h1>OpenAnim Dev Console</h1>
    <div class="toolbar" id="adapters"></div>
  </header>
  <main>
    <aside class="stack">
      <div class="box stack">
        <div class="status" id="status">Loading engine status...</div>
        <button id="reload">Reload project</button>
      </div>
      <div class="box stack">
        <label>LLM provider JSON
          <textarea id="provider">{
  "provider_type": "ollama",
  "base_url": "http://localhost:11434",
  "model": "llama3"
}</textarea>
        </label>
        <label>Prompt
          <textarea id="prompt" placeholder="Describe a scene to compile into Scene IR..."></textarea>
        </label>
        <div class="row">
          <button class="primary" id="generate">Generate scene</button>
          <button id="patch">Patch selected</button>
        </div>
      </div>
      <div class="box stack">
        <label>Scene
          <select id="scene"></select>
        </label>
        <button class="primary" id="render">Render scene</button>
        <video id="video" controls hidden></video>
      </div>
      <div class="box">
        <pre id="log">Ready.</pre>
      </div>
    </aside>
    <section class="stack">
      <div class="toolbar">
        <button class="primary" id="save">Validate and save project.json</button>
      </div>
      <textarea id="project" spellcheck="false"></textarea>
    </section>
  </main>
  <script>
    const $ = (id) => document.getElementById(id);
    let currentProject = null;

    function log(message) {
      $("log").textContent = `${new Date().toLocaleTimeString()}  ${message}\n` + $("log").textContent;
    }

    function setBusy(isBusy) {
      for (const id of ["reload", "generate", "patch", "render", "save"]) {
        $(id).disabled = isBusy;
      }
    }

    async function api(path, options = {}) {
      const res = await fetch(path, {
        headers: { "content-type": "application/json" },
        ...options
      });
      const text = await res.text();
      let data = text;
      try { data = text ? JSON.parse(text) : null; } catch (_) {}
      if (!res.ok) throw new Error(typeof data === "string" ? data : JSON.stringify(data, null, 2));
      return data;
    }

    function refreshScenes() {
      const select = $("scene");
      select.innerHTML = "";
      const scenes = currentProject?.scenes ?? [];
      for (const scene of scenes) {
        const option = document.createElement("option");
        option.value = scene.id;
        option.textContent = `${scene.name || "Untitled"} (${scene.nodes?.length ?? 0} nodes)`;
        select.appendChild(option);
      }
    }

    async function loadStatus() {
      const status = await api("/api/status");
      $("status").textContent = `Engine ${status.engine_version}`;
      $("adapters").innerHTML = Object.entries(status.adapters)
        .map(([name, ok]) => `<span class="pill ${ok ? "ok" : "bad"}">${name}: ${ok ? "ok" : "missing"}</span>`)
        .join("");
    }

    async function loadProject() {
      currentProject = await api("/api/project");
      $("project").value = JSON.stringify(currentProject, null, 2);
      refreshScenes();
      log(`Loaded ${currentProject.scenes?.length ?? 0} scene(s).`);
    }

    async function saveProject() {
      const parsed = JSON.parse($("project").value);
      await api("/api/project", { method: "POST", body: JSON.stringify(parsed) });
      currentProject = parsed;
      refreshScenes();
      log("Saved project.json.");
    }

    function providerPayload() {
      return JSON.parse($("provider").value);
    }

    async function generateScene() {
      const prompt = $("prompt").value.trim();
      if (!prompt) return log("Prompt is empty.");
      const scene = await api("/api/generate", {
        method: "POST",
        body: JSON.stringify({ prompt, provider: providerPayload() })
      });
      log(`Generated scene: ${scene.name}`);
      await loadProject();
    }

    async function patchScene() {
      const prompt = $("prompt").value.trim();
      const scene_id = $("scene").value;
      if (!prompt || !scene_id) return log("Choose a scene and enter a patch prompt.");
      const scene = await api("/api/patch", {
        method: "POST",
        body: JSON.stringify({ prompt, scene_id, provider: providerPayload() })
      });
      log(`Patched scene: ${scene.name}`);
      await loadProject();
    }

    async function renderScene() {
      const scene_id = $("scene").value;
      if (!scene_id) return log("No scene selected.");
      const result = await api("/api/render", {
        method: "POST",
        body: JSON.stringify({ scene_id })
      });
      log(`Rendered: ${result.output_path}`);
      if (result.video_hash) {
        $("video").src = `/api/video/${result.video_hash}`;
        $("video").hidden = false;
      }
    }

    async function run(action) {
      setBusy(true);
      try { await action(); }
      catch (error) { log(error.message || String(error)); }
      finally { setBusy(false); }
    }

    $("reload").onclick = () => run(loadProject);
    $("save").onclick = () => run(saveProject);
    $("generate").onclick = () => run(generateScene);
    $("patch").onclick = () => run(patchScene);
    $("render").onclick = () => run(renderScene);

    run(async () => {
      await loadStatus();
      await loadProject();
    });
  </script>
</body>
</html>
"#;
