#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use oac_core::{
    audio::render_audiobook,
    model::{
        BookMetadata, BookProject, CleanerOptions, ExportConfig, RenderArtifact, RenderProgress,
        TtsConfig, VoiceProfile,
    },
    tts::{list_voice_catalog, run_diagnostics, synthesize_chunk_to_wav, EngineDiagnostics},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::State;
use uuid::Uuid;

struct DesktopState {
    current_project: Arc<Mutex<Option<BookProject>>>,
    render_progress: Arc<Mutex<Option<RenderProgress>>>,
}

#[derive(Serialize)]
struct SystemInfoResponse {
    diagnostics: EngineDiagnostics,
    voices: Vec<VoiceProfile>,
}

#[derive(Serialize)]
struct VoicePreviewResponse {
    audio_base64: String,
    mime_type: String,
    used_engine: String,
}

#[tauri::command]
fn get_system_info() -> SystemInfoResponse {
    SystemInfoResponse {
        diagnostics: run_diagnostics(),
        voices: list_voice_catalog(Path::new("./models")),
    }
}

#[tauri::command]
fn get_project(state: State<'_, DesktopState>) -> Result<BookProject, String> {
    let guard = state.current_project.lock().unwrap();
    guard
        .clone()
        .ok_or_else(|| "Nenhum projeto carregado.".to_string())
}

#[tauri::command]
fn load_sample_project(state: State<'_, DesktopState>) -> Result<BookProject, String> {
    let proj = BookProject::from_sample_pdf().map_err(|e| e.to_string())?;
    *state.current_project.lock().unwrap() = Some(proj.clone());
    Ok(proj)
}

#[tauri::command]
fn import_project_from_path(
    path: String,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let proj =
        BookProject::from_file(Path::new(&path), CleanerOptions::default()).map_err(|e| e.to_string())?;
    *state.current_project.lock().unwrap() = Some(proj.clone());
    Ok(proj)
}

#[tauri::command]
fn update_metadata(
    metadata: BookMetadata,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.metadata = metadata;
    Ok(proj.clone())
}

#[tauri::command]
fn update_cleaner_options(
    options: CleanerOptions,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.re_apply_cleaner(options);
    Ok(proj.clone())
}

#[tauri::command]
fn update_chapter(
    chapter_id: String,
    title: Option<String>,
    cleaned_text: Option<String>,
    enabled: Option<bool>,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.update_chapter(&chapter_id, title, cleaned_text, enabled)
        .map_err(|e| e.to_string())?;
    Ok(proj.clone())
}

#[tauri::command]
fn merge_chapter(chapter_id: String, state: State<'_, DesktopState>) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.merge_chapter_with_next(&chapter_id)
        .map_err(|e| e.to_string())?;
    Ok(proj.clone())
}

#[tauri::command]
fn update_tts_config(
    tts_config: TtsConfig,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.tts_config = tts_config;
    Ok(proj.clone())
}

#[tauri::command]
fn update_export_config(
    export_config: ExportConfig,
    state: State<'_, DesktopState>,
) -> Result<BookProject, String> {
    let mut guard = state.current_project.lock().unwrap();
    let proj = guard.as_mut().ok_or("Projeto não carregado")?;
    proj.export_config = export_config;
    Ok(proj.clone())
}

#[tauri::command]
fn preview_voice(text: String, tts_config: TtsConfig) -> Result<VoicePreviewResponse, String> {
    let tmp_wav = std::env::temp_dir().join(format!("oac_preview_{}.wav", Uuid::new_v4()));
    let used_engine =
        synthesize_chunk_to_wav(&text, &tts_config, &tmp_wav).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&tmp_wav).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_wav);

    Ok(VoicePreviewResponse {
        audio_base64: BASE64.encode(bytes),
        mime_type: "audio/wav".to_string(),
        used_engine,
    })
}

#[tauri::command]
fn start_render(state: State<'_, DesktopState>) -> Result<RenderArtifact, String> {
    let mut project = {
        let guard = state.current_project.lock().unwrap();
        guard.clone().ok_or("Nenhum projeto carregado")?
    };

    let progress_handle = state.render_progress.clone();
    let artifact = render_audiobook(&mut project, &PathBuf::from(".oac-cache"), |prog| {
        *progress_handle.lock().unwrap() = Some(prog);
    })
    .map_err(|e| e.to_string())?;

    *state.current_project.lock().unwrap() = Some(project);
    Ok(artifact)
}

fn main() {
    let initial = BookProject::from_sample_pdf().ok();
    tauri::Builder::default()
        .manage(DesktopState {
            current_project: Arc::new(Mutex::new(initial)),
            render_progress: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_project,
            load_sample_project,
            import_project_from_path,
            update_metadata,
            update_cleaner_options,
            update_chapter,
            merge_chapter,
            update_tts_config,
            update_export_config,
            preview_voice,
            start_render,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar aplicação Tauri");
}

