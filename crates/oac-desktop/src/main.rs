#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod webview2_embedded_loader {
    use std::ffi::{c_void, CString};
    use std::os::windows::ffi::OsStrExt;
    use std::sync::OnceLock;

    const WEBVIEW2_LOADER_DLL: &[u8] = include_bytes!("../assets/WebView2Loader.dll");

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
        fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const i8) -> *mut c_void;
    }

    fn get_loader_module() -> *mut c_void {
        static MODULE: OnceLock<usize> = OnceLock::new();
        *MODULE.get_or_init(|| {
            let dll_path = std::env::temp_dir().join("oac_WebView2Loader_x64.dll");
            let needs_write = match std::fs::metadata(&dll_path) {
                Ok(m) => m.len() != WEBVIEW2_LOADER_DLL.len() as u64,
                Err(_) => true,
            };
            if needs_write {
                let _ = std::fs::write(&dll_path, WEBVIEW2_LOADER_DLL);
            }
            let wide: Vec<u16> = dll_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe { LoadLibraryW(wide.as_ptr()) as usize }
        }) as *mut c_void
    }

    unsafe fn get_proc(name: &str) -> *mut c_void {
        let hmod = get_loader_module();
        if hmod.is_null() {
            return std::ptr::null_mut();
        }
        let cname = CString::new(name).unwrap();
        GetProcAddress(hmod, cname.as_ptr())
    }

    #[no_mangle]
    pub unsafe extern "system" fn CompareBrowserVersions(
        version1: *const u16,
        version2: *const u16,
        result: *mut i32,
    ) -> i32 {
        type FnType = unsafe extern "system" fn(*const u16, *const u16, *mut i32) -> i32;
        let ptr = get_proc("CompareBrowserVersions");
        if ptr.is_null() {
            return -2147467259i32;
        }
        let func: FnType = std::mem::transmute(ptr);
        func(version1, version2, result)
    }

    #[no_mangle]
    pub unsafe extern "system" fn CreateCoreWebView2Environment(
        environmentcreatedhandler: *mut c_void,
    ) -> i32 {
        type FnType = unsafe extern "system" fn(*mut c_void) -> i32;
        let ptr = get_proc("CreateCoreWebView2Environment");
        if ptr.is_null() {
            return -2147467259i32;
        }
        let func: FnType = std::mem::transmute(ptr);
        func(environmentcreatedhandler)
    }

    #[no_mangle]
    pub unsafe extern "system" fn CreateCoreWebView2EnvironmentWithOptions(
        browserexecutablefolder: *const u16,
        userdatafolder: *const u16,
        environmentoptions: *mut c_void,
        environmentcreatedhandler: *mut c_void,
    ) -> i32 {
        type FnType =
            unsafe extern "system" fn(*const u16, *const u16, *mut c_void, *mut c_void) -> i32;
        let ptr = get_proc("CreateCoreWebView2EnvironmentWithOptions");
        if ptr.is_null() {
            return -2147467259i32;
        }
        let func: FnType = std::mem::transmute(ptr);
        func(
            browserexecutablefolder,
            userdatafolder,
            environmentoptions,
            environmentcreatedhandler,
        )
    }

    #[no_mangle]
    pub unsafe extern "system" fn GetAvailableCoreWebView2BrowserVersionString(
        browserexecutablefolder: *const u16,
        versioninfo: *mut *mut u16,
    ) -> i32 {
        type FnType = unsafe extern "system" fn(*const u16, *mut *mut u16) -> i32;
        let ptr = get_proc("GetAvailableCoreWebView2BrowserVersionString");
        if ptr.is_null() {
            return -2147467259i32;
        }
        let func: FnType = std::mem::transmute(ptr);
        func(browserexecutablefolder, versioninfo)
    }

    #[no_mangle]
    pub unsafe extern "system" fn GetAvailableCoreWebView2BrowserVersionStringWithOptions(
        browserexecutablefolder: *const u16,
        environmentoptions: *mut c_void,
        versioninfo: *mut *mut u16,
    ) -> i32 {
        type FnType = unsafe extern "system" fn(*const u16, *mut c_void, *mut *mut u16) -> i32;
        let ptr = get_proc("GetAvailableCoreWebView2BrowserVersionStringWithOptions");
        if ptr.is_null() {
            return -2147467259i32;
        }
        let func: FnType = std::mem::transmute(ptr);
        func(browserexecutablefolder, environmentoptions, versioninfo)
    }
}

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use oac_core::{
    audio::render_audiobook,
    model::{
        BookMetadata, BookProject, CleanerOptions, ExportConfig, RenderProgress, SourceFormat,
        TtsConfig,
    },
    tts::{
        extract_embedded_bundle_if_present, list_voice_catalog, run_diagnostics, run_setup_wizard,
        synthesize_chunk_to_wav, WizardProgress,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use uuid::Uuid;
use wry::{http::Response, WebViewBuilder};

const INDEX_HTML: &str = include_str!("../../../ui/dist/index.html");
const APP_JS: &str = include_str!("../../../ui/dist/assets/app.js");
const APP_CSS: &str = include_str!("../../../ui/dist/assets/app.css");

const IPC_INIT_SCRIPT: &str = r#"
(function() {
  window.__OAC_PENDING__ = new Map();
  window.__OAC_RESOLVE__ = function(id, ok, payload) {
    const entry = window.__OAC_PENDING__.get(id);
    if (!entry) return;
    window.__OAC_PENDING__.delete(id);
    if (ok) {
      entry.resolve(payload);
    } else {
      const msg = typeof payload === 'string' ? payload : (payload && payload.error ? payload.error : JSON.stringify(payload));
      entry.reject(new Error(msg));
    }
  };
  window.__OAC_DESKTOP__ = {
    invoke: function(cmd, args) {
      return new Promise(function(resolve, reject) {
        const id = Math.random().toString(36).slice(2) + Date.now().toString(36);
        window.__OAC_PENDING__.set(id, { resolve: resolve, reject: reject });
        window.ipc.postMessage(JSON.stringify({ id: id, cmd: cmd, args: args || {} }));
      });
    }
  };
})();
"#;

#[derive(Clone)]
struct DesktopState {
    current_project: Arc<Mutex<BookProject>>,
    render_progress: Arc<Mutex<Option<RenderProgress>>>,
    wizard_progress: Arc<Mutex<Option<WizardProgress>>>,
    last_rendered_file: Arc<Mutex<Option<PathBuf>>>,
}

#[derive(Deserialize)]
struct IpcEnvelope {
    id: String,
    cmd: String,
    #[serde(default)]
    args: Value,
}

enum UserEvent {
    IpcReply {
        id: String,
        ok: bool,
        payload: Value,
    },
    #[cfg(target_os = "linux")]
    PickDirectory {
        id: String,
        current_dir: Option<String>,
    },
}

#[derive(Serialize)]
struct VoicePreviewResponse {
    audio_base64: String,
    mime_type: String,
    used_engine: String,
}

fn default_audiobooks_dir() -> String {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join("Audiobooks")
            .to_string_lossy()
            .to_string();
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile)
            .join("Audiobooks")
            .to_string_lossy()
            .to_string();
    }
    "./output".to_string()
}

fn open_directory_in_os(path: &str) -> Result<(), String> {
    let dir = Path::new(path);
    if !dir.exists() {
        let _ = std::fs::create_dir_all(dir);
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn pick_directory_non_linux(current_dir: Option<&str>) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let initial = current_dir.unwrap_or("").replace('\'', "''");
        let ps_script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $f = New-Object System.Windows.Forms.FolderBrowserDialog; \
             $f.Description = 'Escolher Pasta de Saída do Audiobook'; \
             if ('{initial}' -ne '' -and (Test-Path '{initial}')) {{ $f.SelectedPath = '{initial}' }}; \
             if ($f.ShowDialog() -eq 'OK') {{ [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Write-Output $f.SelectedPath }}"
        );
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Sta", "-Command", &ps_script])
            .output()
        {
            let selected = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !selected.is_empty() {
                return Some(selected);
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        let _ = current_dir;
        if let Ok(out) = std::process::Command::new("osascript")
            .args([
                "-e",
                "POSIX path of (choose folder with prompt \"Escolher Pasta de Saída do Audiobook\")",
            ])
            .output()
        {
            let selected = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !selected.is_empty() {
                return Some(selected);
            }
        }
        None
    }
}

fn handle_ipc_command(state: &DesktopState, cmd: &str, args: Value) -> Result<Value, String> {
    match cmd {
        "get_system_info" => {
            let diagnostics = run_diagnostics();
            let voices = list_voice_catalog(Path::new("./models"));
            Ok(json!({
                "diagnostics": diagnostics,
                "voices": voices,
            }))
        }

        "get_project" => {
            let proj = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao acessar estado do projeto".to_string())?
                .clone();
            Ok(serde_json::to_value(proj).map_err(|e| e.to_string())?)
        }

        "load_sample_project" => {
            let prev = state.current_project.lock().ok().map(|p| p.clone());
            let mut proj = BookProject::from_sample_pdf().map_err(|e| e.to_string())?;
            if let Some(prev_proj) = prev {
                proj.export_config = prev_proj.export_config;
                proj.tts_config = prev_proj.tts_config;
            } else {
                proj.export_config.output_dir = default_audiobooks_dir();
            }
            *state
                .current_project
                .lock()
                .map_err(|_| "Falha ao atualizar projeto".to_string())? = proj.clone();
            Ok(serde_json::to_value(proj).map_err(|e| e.to_string())?)
        }

        "upload_book_file" => {
            let filename = args
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("livro.pdf");
            let data_base64 = args
                .get("data_base64")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Conteúdo do arquivo ausente".to_string())?;

            let bytes = BASE64
                .decode(data_base64)
                .map_err(|e| format!("Falha ao decodificar arquivo: {e}"))?;

            let prev = state.current_project.lock().ok().map(|p| p.clone());
            let cleaner_opts = prev
                .as_ref()
                .map(|p| p.cleaner_options.clone())
                .unwrap_or_default();

            let path = Path::new(filename);
            let format = SourceFormat::from_path(path).unwrap_or(SourceFormat::Pdf);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Livro Importado");

            let mut proj = BookProject::from_bytes(&bytes, format, Some(stem), cleaner_opts)
                .map_err(|e| e.to_string())?;
            if let Some(prev_proj) = prev {
                proj.export_config = prev_proj.export_config;
                proj.tts_config = prev_proj.tts_config;
            }
            *state
                .current_project
                .lock()
                .map_err(|_| "Falha ao atualizar projeto".to_string())? = proj.clone();
            Ok(serde_json::to_value(proj).map_err(|e| e.to_string())?)
        }

        "import_project_from_path" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Caminho não informado".to_string())?;
            let prev = state.current_project.lock().ok().map(|p| p.clone());
            let cleaner_opts = prev
                .as_ref()
                .map(|p| p.cleaner_options.clone())
                .unwrap_or_default();
            let mut proj =
                BookProject::from_file(Path::new(path), cleaner_opts).map_err(|e| e.to_string())?;
            if let Some(prev_proj) = prev {
                proj.export_config = prev_proj.export_config;
                proj.tts_config = prev_proj.tts_config;
            }
            *state
                .current_project
                .lock()
                .map_err(|_| "Falha ao salvar projeto".to_string())? = proj.clone();
            Ok(serde_json::to_value(proj).map_err(|e| e.to_string())?)
        }

        "update_metadata" => {
            let metadata: BookMetadata = serde_json::from_value(
                args.get("metadata").cloned().unwrap_or(args),
            )
            .map_err(|e| e.to_string())?;
            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao atualizar metadados".to_string())?;
            guard.metadata = metadata;
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "update_cleaner_options" => {
            let options: CleanerOptions = serde_json::from_value(
                args.get("options").cloned().unwrap_or(args),
            )
            .map_err(|e| e.to_string())?;
            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao reaplicar limpeza".to_string())?;
            guard.re_apply_cleaner(options);
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "update_chapter" => {
            let chapter_id = args
                .get("chapter_id")
                .or_else(|| args.get("chapterId"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "chapter_id ausente".to_string())?
                .to_string();
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let cleaned_text = args
                .get("cleaned_text")
                .or_else(|| args.get("cleanedText"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let enabled = args.get("enabled").and_then(|v| v.as_bool());

            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao atualizar capítulo".to_string())?;
            guard
                .update_chapter(&chapter_id, title, cleaned_text, enabled)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "merge_chapter" => {
            let chapter_id = args
                .get("chapter_id")
                .or_else(|| args.get("chapterId"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "chapter_id ausente".to_string())?;
            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao mesclar capítulos".to_string())?;
            guard
                .merge_chapter_with_next(chapter_id)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "add_custom_chapter" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Novo Capítulo")
                .to_string();
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao adicionar capítulo".to_string())?;
            guard.add_custom_chapter(title, text);
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "update_tts_config" => {
            let tts_config: TtsConfig = serde_json::from_value(
                args.get("tts_config")
                    .or_else(|| args.get("ttsConfig"))
                    .cloned()
                    .unwrap_or(args),
            )
            .map_err(|e| e.to_string())?;
            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao salvar configuração TTS".to_string())?;
            guard.tts_config = tts_config;
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        "update_export_config" => {
            let export_config: ExportConfig = serde_json::from_value(
                args.get("export_config")
                    .or_else(|| args.get("exportConfig"))
                    .cloned()
                    .unwrap_or(args),
            )
            .map_err(|e| e.to_string())?;
            let mut guard = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao salvar configuração de exportação".to_string())?;
            guard.export_config = export_config;
            Ok(serde_json::to_value(guard.clone()).map_err(|e| e.to_string())?)
        }

        #[cfg(not(target_os = "linux"))]
        "pick_output_dir" => {
            let current_dir = args.get("current_dir").and_then(|v| v.as_str());
            let chosen = pick_directory_non_linux(current_dir);
            Ok(json!(chosen))
        }

        "open_output_dir" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("./output");
            open_directory_in_os(path)?;
            Ok(json!({ "ok": true }))
        }

        "preview_voice" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("Olá! Este é um teste de voz local do Open Audiobook Creator.")
                .to_string();
            let tts_config: TtsConfig = serde_json::from_value(
                args.get("tts_config")
                    .or_else(|| args.get("ttsConfig"))
                    .cloned()
                    .ok_or_else(|| "tts_config ausente".to_string())?,
            )
            .map_err(|e| e.to_string())?;

            let tmp_wav = std::env::temp_dir().join(format!("oac_preview_{}.wav", Uuid::new_v4()));
            let used_engine =
                synthesize_chunk_to_wav(&text, &tts_config, &tmp_wav).map_err(|e| e.to_string())?;
            let bytes = std::fs::read(&tmp_wav).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&tmp_wav);

            let resp = VoicePreviewResponse {
                audio_base64: BASE64.encode(bytes),
                mime_type: "audio/wav".to_string(),
                used_engine,
            };
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }

        "start_render" => {
            let mut project = state
                .current_project
                .lock()
                .map_err(|_| "Falha ao ler projeto".to_string())?
                .clone();

            let cache_dir = std::env::temp_dir().join("oac-cache");
            let progress_handle = state.render_progress.clone();
            let artifact = render_audiobook(&mut project, &cache_dir, |prog| {
                if let Ok(mut g) = progress_handle.lock() {
                    *g = Some(prog);
                }
            })
            .map_err(|e| e.to_string())?;

            if let Ok(mut last) = state.last_rendered_file.lock() {
                *last = Some(PathBuf::from(&artifact.primary_output_path));
            }
            if let Ok(mut guard) = state.current_project.lock() {
                *guard = project;
            }
            Ok(serde_json::to_value(artifact).map_err(|e| e.to_string())?)
        }

        "get_wizard_progress" => {
            let prog = state
                .wizard_progress
                .lock()
                .ok()
                .and_then(|g| g.clone());
            Ok(serde_json::to_value(prog).map_err(|e| e.to_string())?)
        }

        "run_setup_wizard" => {
            let wiz_handle = state.wizard_progress.clone();
            let res = run_setup_wizard(|percent, step| {
                if let Ok(mut g) = wiz_handle.lock() {
                    *g = Some(WizardProgress {
                        percent,
                        step: step.to_string(),
                        finished: percent >= 100.0,
                        error: None,
                    });
                }
            });

            match res {
                Ok(diagnostics) => {
                    let voices = list_voice_catalog(Path::new("./models"));
                    Ok(json!({
                        "diagnostics": diagnostics,
                        "voices": voices,
                    }))
                }
                Err(err) => {
                    if let Ok(mut g) = state.wizard_progress.lock() {
                        *g = Some(WizardProgress {
                            percent: 0.0,
                            step: "Erro durante a instalação".to_string(),
                            finished: true,
                            error: Some(err.to_string()),
                        });
                    }
                    Err(err.to_string())
                }
            }
        }

        other => Err(format!("Comando IPC desconhecido: {other}")),
    }
}

fn build_asset_response(uri_path: &str) -> Response<Cow<'static, [u8]>> {
    let clean_path = uri_path.trim_start_matches('/');
    match clean_path {
        "" | "index.html" => Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(INDEX_HTML.as_bytes()))
            .unwrap(),
        "assets/app.js" => Response::builder()
            .status(200)
            .header("Content-Type", "text/javascript; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(APP_JS.as_bytes()))
            .unwrap(),
        "assets/app.css" => Response::builder()
            .status(200)
            .header("Content-Type", "text/css; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(APP_CSS.as_bytes()))
            .unwrap(),
        _ => Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(INDEX_HTML.as_bytes()))
            .unwrap(),
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Evita falhas de DMA-BUF do WebKit2GTK em placas AMD Radeon / Wayland
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter("info,lopdf=error")
        .with_target(false)
        .init();

    // Se o executável (.exe) contiver o pacote All-in-One embutido, extrai automaticamente no 1º uso
    let _ = extract_embedded_bundle_if_present();

    let mut initial_project = BookProject::from_sample_pdf().or_else(|_| {
        BookProject::from_bytes(
            b"Capitulo 1\n\nBem-vindo ao Open Audiobook Creator.",
            SourceFormat::Txt,
            Some("Projeto Inicial"),
            CleanerOptions::default(),
        )
    })?;
    initial_project.export_config.output_dir = default_audiobooks_dir();

    let state = DesktopState {
        current_project: Arc::new(Mutex::new(initial_project)),
        render_progress: Arc::new(Mutex::new(None)),
        wizard_progress: Arc::new(Mutex::new(None)),
        last_rendered_file: Arc::new(Mutex::new(None)),
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Open Audiobook Creator")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(960.0, 640.0))
        .build(&event_loop)?;

    let ipc_state = state.clone();
    let builder = WebViewBuilder::new()
        .with_custom_protocol("oac".into(), move |_webview_id, request| {
            let path = request.uri().path();
            build_asset_response(path)
        })
        .with_initialization_script(IPC_INIT_SCRIPT)
        .with_ipc_handler(move |request| {
            let body = request.body().clone();
            let parsed: Result<IpcEnvelope, _> = serde_json::from_str(&body);
            if let Ok(env) = parsed {
                tracing::info!("IPC nativo recebido: {}", env.cmd);

                #[cfg(target_os = "linux")]
                if env.cmd == "pick_output_dir" {
                    let current_dir = env
                        .args
                        .get("current_dir")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let _ = proxy.send_event(UserEvent::PickDirectory {
                        id: env.id,
                        current_dir,
                    });
                    return;
                }

                let st = ipc_state.clone();
                let prx = proxy.clone();
                std::thread::spawn(move || {
                    let result = handle_ipc_command(&st, &env.cmd, env.args);
                    let (ok, payload) = match result {
                        Ok(val) => (true, val),
                        Err(err) => (false, json!({ "error": err })),
                    };
                    let _ = prx.send_event(UserEvent::IpcReply {
                        id: env.id,
                        ok,
                        payload,
                    });
                });
            }
        })
        .with_url("oac://localhost/index.html");

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window
            .default_vbox()
            .ok_or_else(|| anyhow::anyhow!("Falha ao obter container GTK da janela nativa"))?;
        builder.build_gtk(vbox)?
    };

    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            #[cfg(target_os = "linux")]
            Event::UserEvent(UserEvent::PickDirectory { id, current_dir }) => {
                use gtk::prelude::*;
                use tao::platform::unix::WindowExtUnix;

                let gtk_win = window.gtk_window();
                let dialog = gtk::FileChooserDialog::with_buttons(
                    Some("Escolher Pasta de Saída do Audiobook"),
                    Some(gtk_win),
                    gtk::FileChooserAction::SelectFolder,
                    &[
                        ("Cancelar", gtk::ResponseType::Cancel),
                        ("Selecionar Pasta", gtk::ResponseType::Accept),
                    ],
                );
                if let Some(dir) = current_dir {
                    let p = Path::new(&dir);
                    if p.exists() {
                        let _ = dialog.set_current_folder(p);
                    } else if let Some(parent) = p.parent() {
                        if parent.exists() {
                            let _ = dialog.set_current_folder(parent);
                        }
                    }
                }
                let chosen = if dialog.run() == gtk::ResponseType::Accept {
                    dialog.filename().map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };
                unsafe {
                    dialog.destroy();
                }

                if let (Ok(id_json), Ok(payload_json)) = (
                    serde_json::to_string(&id),
                    serde_json::to_string(&json!(chosen)),
                ) {
                    let script =
                        format!("window.__OAC_RESOLVE__({id_json}, true, {payload_json});");
                    let _ = webview.evaluate_script(&script);
                }
            }
            Event::UserEvent(UserEvent::IpcReply { id, ok, payload }) => {
                if let Ok(id_json) = serde_json::to_string(&id) {
                    if let Ok(payload_json) = serde_json::to_string(&payload) {
                        let script = format!(
                            "window.__OAC_RESOLVE__({id_json}, {ok}, {payload_json});"
                        );
                        let _ = webview.evaluate_script(&script);
                    }
                }
            }
            _ => {}
        }
    });
}
