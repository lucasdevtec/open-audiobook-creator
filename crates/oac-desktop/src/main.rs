#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use oac_core::{
    audio::render_audiobook,
    model::{
        BookMetadata, BookProject, CleanerOptions, ExportConfig, RenderProgress, SourceFormat,
        TtsConfig,
    },
    tts::{list_voice_catalog, run_diagnostics, synthesize_chunk_to_wav},
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
}

#[derive(Serialize)]
struct VoicePreviewResponse {
    audio_base64: String,
    mime_type: String,
    used_engine: String,
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
            let proj = BookProject::from_sample_pdf().map_err(|e| e.to_string())?;
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

            let cleaner_opts = state
                .current_project
                .lock()
                .map(|p| p.cleaner_options.clone())
                .unwrap_or_default();

            let path = Path::new(filename);
            let format = SourceFormat::from_path(path).unwrap_or(SourceFormat::Pdf);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Livro Importado");

            let proj = BookProject::from_bytes(&bytes, format, Some(stem), cleaner_opts)
                .map_err(|e| e.to_string())?;
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
            let cleaner_opts = state
                .current_project
                .lock()
                .map(|p| p.cleaner_options.clone())
                .unwrap_or_default();
            let proj =
                BookProject::from_file(Path::new(path), cleaner_opts).map_err(|e| e.to_string())?;
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

            let progress_handle = state.render_progress.clone();
            let artifact = render_audiobook(&mut project, &PathBuf::from(".oac-cache"), |prog| {
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

    let initial_project = BookProject::from_sample_pdf().or_else(|_| {
        BookProject::from_bytes(
            b"Capitulo 1\n\nBem-vindo ao Open Audiobook Creator.",
            SourceFormat::Txt,
            Some("Projeto Inicial"),
            CleanerOptions::default(),
        )
    })?;

    let state = DesktopState {
        current_project: Arc::new(Mutex::new(initial_project)),
        render_progress: Arc::new(Mutex::new(None)),
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
