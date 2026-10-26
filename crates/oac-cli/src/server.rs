use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use oac_core::{
    audio::render_audiobook,
    model::{
        BookMetadata, BookProject, CleanerOptions, ExportConfig, RenderArtifact, RenderProgress,
        SourceFormat, TtsConfig, VoiceProfile,
    },
    tts::{list_voice_catalog, run_diagnostics, synthesize_chunk_to_wav, EngineDiagnostics},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub current_project: Arc<Mutex<Option<BookProject>>>,
    pub render_progress: Arc<Mutex<Option<RenderProgress>>>,
}

impl Default for AppState {
    fn default() -> Self {
        let initial = BookProject::from_sample_pdf().ok();
        Self {
            current_project: Arc::new(Mutex::new(initial)),
            render_progress: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Serialize)]
pub struct SystemInfoResponse {
    pub diagnostics: EngineDiagnostics,
    pub voices: Vec<VoiceProfile>,
}

#[derive(Deserialize)]
pub struct UpdateChapterRequest {
    pub chapter_id: String,
    pub title: Option<String>,
    pub cleaned_text: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
pub struct MergeChapterRequest {
    pub chapter_id: String,
}

#[derive(Deserialize)]
pub struct AddChapterRequest {
    pub title: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct VoicePreviewRequest {
    pub text: String,
    pub tts_config: TtsConfig,
}

#[derive(Serialize)]
pub struct VoicePreviewResponse {
    pub audio_base64: String,
    pub mime_type: String,
    pub used_engine: String,
}

#[derive(Deserialize)]
pub struct ImportPathRequest {
    pub path: String,
}

const EMBEDDED_INDEX_HTML: &str = include_str!("../../../ui/dist/index.html");
const EMBEDDED_APP_JS: &str = include_str!("../../../ui/dist/assets/app.js");
const EMBEDDED_APP_CSS: &str = include_str!("../../../ui/dist/assets/app.css");

pub async fn run_http_server(port: u16, open_window: bool) -> anyhow::Result<()> {
    let state = AppState::default();

    let app = Router::new()
        .route("/", get(serve_index_html))
        .route("/index.html", get(serve_index_html))
        .route("/assets/app.js", get(serve_app_js))
        .route("/assets/app.css", get(serve_app_css))
        .route("/api/system", get(get_system_info))
        .route("/api/project", get(get_project))
        .route("/api/project/sample", post(load_sample_project))
        .route("/api/project/import-path", post(import_project_from_path))
        .route("/api/project/upload", post(upload_book_file))
        .route("/api/project/metadata", post(update_metadata))
        .route("/api/project/cleaner", post(update_cleaner_options))
        .route("/api/project/chapter/update", post(update_chapter))
        .route("/api/project/chapter/merge", post(merge_chapter))
        .route("/api/project/chapter/add", post(add_chapter))
        .route("/api/project/tts", post(update_tts_config))
        .route("/api/project/export-config", post(update_export_config))
        .route("/api/tts/preview", post(preview_voice))
        .route("/api/audio/data", post(get_audio_data))
        .route("/api/render/start", post(start_render))
        .route("/api/render/progress", get(get_render_progress))
        .layer(DefaultBodyLimit::max(120 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let url = format!("http://{addr}");
    println!("🚀 Open Audiobook Creator Studio ativo em {url}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    if open_window {
        let url_clone = url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            open_native_app_window(&url_clone);
        });
    }

    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_index_html() -> Html<&'static str> {
    Html(EMBEDDED_INDEX_HTML)
}

async fn serve_app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        EMBEDDED_APP_JS,
    )
}

async fn serve_app_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        EMBEDDED_APP_CSS,
    )
}

fn open_native_app_window(url: &str) {
    #[cfg(target_os = "windows")]
    {
        // Tenta abrir em modo janela de aplicativo nativa (Edge WebView / Chrome --app)
        let app_arg = format!("--app={url}");
        let launched = std::process::Command::new("cmd")
            .args(["/C", "start", "", "msedge", &app_arg])
            .spawn()
            .is_ok();
        if !launched {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        let app_arg = format!("--app={url}");
        for browser in [
            "google-chrome",
            "chromium",
            "chromium-browser",
            "brave-browser",
            "microsoft-edge",
        ] {
            if std::process::Command::new(browser)
                .arg(&app_arg)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return;
            }
        }
        let _ = std::process::Command::new("xdg-open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
}

async fn get_system_info() -> Json<SystemInfoResponse> {
    let diagnostics = run_diagnostics();
    let voices = list_voice_catalog(Path::new("./models"));
    Json(SystemInfoResponse {
        diagnostics,
        voices,
    })
}

async fn get_project(
    State(state): State<AppState>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let guard = state.current_project.lock().unwrap();
    match &*guard {
        Some(proj) => Ok(Json(proj.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            "Nenhum projeto carregado.".to_string(),
        )),
    }
}

async fn load_sample_project(
    State(state): State<AppState>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let proj =
        BookProject::from_sample_pdf().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    *state.current_project.lock().unwrap() = Some(proj.clone());
    Ok(Json(proj))
}

async fn import_project_from_path(
    State(state): State<AppState>,
    Json(req): Json<ImportPathRequest>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let path = PathBuf::from(&req.path);
    let proj = BookProject::from_file(&path, CleanerOptions::default())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    *state.current_project.lock().unwrap() = Some(proj.clone());
    Ok(Json(proj))
}

async fn upload_book_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = String::from("documento.pdf");

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.file_name() {
            file_name = name.to_string();
        }
        if let Ok(bytes) = field.bytes().await {
            if !bytes.is_empty() {
                file_bytes = Some(bytes.to_vec());
                break;
            }
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Nenhum arquivo enviado no upload.".to_string(),
        )
    })?;

    let path = Path::new(&file_name);
    let format = SourceFormat::from_path(path).unwrap_or(SourceFormat::Pdf);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Livro Importado");

    let proj = BookProject::from_bytes(&bytes, format, Some(stem), CleanerOptions::default())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    *state.current_project.lock().unwrap() = Some(proj.clone());
    Ok(Json(proj))
}

async fn update_metadata(
    State(state): State<AppState>,
    Json(metadata): Json<BookMetadata>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.metadata = metadata;
    Ok(Json(proj.clone()))
}

async fn update_cleaner_options(
    State(state): State<AppState>,
    Json(options): Json<CleanerOptions>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.re_apply_cleaner(options);
    Ok(Json(proj.clone()))
}

async fn update_chapter(
    State(state): State<AppState>,
    Json(req): Json<UpdateChapterRequest>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.update_chapter(&req.chapter_id, req.title, req.cleaned_text, req.enabled)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(proj.clone()))
}

async fn merge_chapter(
    State(state): State<AppState>,
    Json(req): Json<MergeChapterRequest>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.merge_chapter_with_next(&req.chapter_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(proj.clone()))
}

async fn add_chapter(
    State(state): State<AppState>,
    Json(req): Json<AddChapterRequest>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.add_custom_chapter(req.title, req.text);
    Ok(Json(proj.clone()))
}

async fn update_tts_config(
    State(state): State<AppState>,
    Json(tts_config): Json<TtsConfig>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.tts_config = tts_config;
    Ok(Json(proj.clone()))
}

async fn update_export_config(
    State(state): State<AppState>,
    Json(export_config): Json<ExportConfig>,
) -> Result<Json<BookProject>, (StatusCode, String)> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Projeto não encontrado".to_string()))?;
    proj.export_config = export_config;
    Ok(Json(proj.clone()))
}

async fn preview_voice(
    Json(req): Json<VoicePreviewRequest>,
) -> Result<Json<VoicePreviewResponse>, (StatusCode, String)> {
    let tmp_wav = std::env::temp_dir().join(format!("oac_preview_{}.wav", Uuid::new_v4()));
    let used_engine = synthesize_chunk_to_wav(&req.text, &req.tts_config, &tmp_wav)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let bytes = std::fs::read(&tmp_wav)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let _ = std::fs::remove_file(&tmp_wav);

    Ok(Json(VoicePreviewResponse {
        audio_base64: BASE64.encode(bytes),
        mime_type: "audio/wav".to_string(),
        used_engine,
    }))
}

#[derive(Deserialize)]
pub struct AudioDataRequest {
    pub path: String,
}

async fn get_audio_data(
    Json(payload): Json<AudioDataRequest>,
) -> Result<Json<VoicePreviewResponse>, (StatusCode, String)> {
    let bytes = std::fs::read(&payload.path)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Falha ao ler áudio: {e}")))?;
    let mime = if payload.path.ends_with(".m4b") || payload.path.ends_with(".mp4") {
        "audio/mp4".to_string()
    } else if payload.path.ends_with(".mp3") {
        "audio/mpeg".to_string()
    } else {
        "audio/wav".to_string()
    };
    Ok(Json(VoicePreviewResponse {
        audio_base64: BASE64.encode(bytes),
        mime_type: mime,
        used_engine: "player".to_string(),
    }))
}

async fn start_render(
    State(state): State<AppState>,
) -> Result<Json<RenderArtifact>, (StatusCode, String)> {
    let mut project = {
        let guard = state.current_project.lock().unwrap();
        guard
            .clone()
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Nenhum projeto carregado".to_string()))?
    };

    let progress_handle = state.render_progress.clone();
    let cache_dir = PathBuf::from(".oac-cache");

    let artifact = render_audiobook(&mut project, &cache_dir, |prog| {
        *progress_handle.lock().unwrap() = Some(prog);
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    *state.current_project.lock().unwrap() = Some(project);
    Ok(Json(artifact))
}

async fn get_render_progress(
    State(state): State<AppState>,
) -> Json<Option<RenderProgress>> {
    let guard = state.render_progress.lock().unwrap();
    Json(guard.clone())
}

