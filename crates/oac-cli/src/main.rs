mod server;

use clap::{Parser, Subcommand};
use oac_core::{
    audio::render_audiobook,
    model::{BookProject, CleanerOptions, ExportFormat, NarrationStyle, TtsEngineKind},
    parser::pdf::generate_sample_pdf_bytes,
    tts::{list_voice_catalog, run_diagnostics},
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "oac",
    version,
    about = "Open Audiobook Creator — Conversor Open Source de PDF/EPUB/TXT para Audiobook (M4B/MP3)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Extrai capítulos de um arquivo PDF, EPUB ou TXT e salva um projeto editável (.oac.json)
    Extract {
        /// Caminho do arquivo de entrada (PDF, EPUB ou TXT)
        #[arg(short, long)]
        input: PathBuf,

        /// Caminho para salvar o arquivo de projeto (.oac.json)
        #[arg(short, long, default_value = "projeto.oac.json")]
        output: PathBuf,
    },

    /// Gera um PDF de demonstração e cria um audiobook completo imediatamente
    Demo {
        /// Diretório de saída para o PDF e o audiobook gerado
        #[arg(short, long, default_value = "./output")]
        output_dir: PathBuf,
    },

    /// Renderiza um arquivo de entrada (PDF/EPUB/TXT) ou projeto (.oac.json) em Audiobook (M4B/MP3/WAV)
    Render {
        /// Arquivo de entrada (.pdf, .epub, .txt ou .oac.json)
        #[arg(short, long)]
        input: PathBuf,

        /// Formato de saída: m4b, mp3, mp3-chapters ou wav
        #[arg(short, long, default_value = "m4b")]
        format: String,

        /// Motor TTS: kokoro, piper, xtts ou builtin
        #[arg(short, long, default_value = "kokoro")]
        engine: String,

        /// Identificador da voz (ex: kokoro-blend_cronista, kokoro-blend_bardo, kokoro-pm_santa, pt_BR-faber-medium)
        #[arg(short, long, default_value = "kokoro-blend_cronista")]
        voice: String,

        /// Estilo/Tom de narração: storyteller, professor, dramatic ou natural
        #[arg(short, long, default_value = "storyteller")]
        style: String,

        /// Diretório de saída
        #[arg(short, long, default_value = "./output")]
        output_dir: String,
    },

    /// Lista motores TTS instalados e o catálogo de vozes locais suportadas
    Voices {
        /// Diretório onde os modelos ONNX estão armazenados
        #[arg(short, long, default_value = "./models")]
        models_dir: PathBuf,
    },

    /// Inicia o servidor local de estúdio para conectar com a interface gráfica
    Serve {
        /// Porta HTTP local
        #[arg(short, long, default_value_t = 14200)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,lopdf=error")
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            server::run_http_server(14200, true).await?;
        }

        Some(Commands::Extract { input, output }) => {
            println!("📖 Extraindo e limpando documento: {}", input.display());
            let project = BookProject::from_file(&input, CleanerOptions::default())?;
            project.save_to_file(&output)?;
            println!("✅ Projeto salvo em: {}", output.display());
            println!("   Título: {}", project.metadata.title);
            println!("   Autor: {}", project.metadata.author);
            println!(
                "   Capítulos detectados: {}",
                project.cleaning_report.chapters_detected
            );
            println!(
                "   Números de página removidos: {}",
                project.cleaning_report.page_numbers_removed
            );
            println!(
                "   Hifenizações corrigidas: {}",
                project.cleaning_report.hyphenations_fixed
            );
            println!("   Total de palavras: {}", project.cleaning_report.total_words);
        }

        Some(Commands::Demo { output_dir }) => {
            std::fs::create_dir_all(&output_dir)?;
            let sample_pdf_path = output_dir.join("livro_exemplo.pdf");
            let pdf_bytes = generate_sample_pdf_bytes()?;
            std::fs::write(&sample_pdf_path, &pdf_bytes)?;
            println!("📄 PDF de demonstração gerado em: {}", sample_pdf_path.display());

            let mut project =
                BookProject::from_file(&sample_pdf_path, CleanerOptions::default())?;
            project.export_config.output_dir = output_dir.to_string_lossy().to_string();
            project.export_config.format = ExportFormat::M4b;

            println!(
                "🧹 Limpeza concluída: {} capítulos extraídos ({} palavras).",
                project.chapters.len(),
                project.cleaning_report.total_words
            );

            let artifact = render_audiobook(&mut project, Path::new(".oac-cache"), |prog| {
                println!(
                    "   [{:3.0}%] {} (Trecho {}/{})",
                    prog.percent, prog.status, prog.completed_chunks, prog.total_chunks
                );
            })?;

            println!("\n🎧 Audiobook gerado com sucesso!");
            println!("   Arquivo principal: {}", artifact.primary_output_path);
            println!("   Duração total: {:.1}s", artifact.total_duration_secs);
            println!("   Motor utilizado: {}", artifact.used_engine);
            println!("   Nota do codificador: {}", artifact.encoder_note);
            println!("   Capítulos embutidos:");
            for marker in &artifact.chapter_markers {
                println!(
                    "     - #{}: {} ({}ms -> {}ms)",
                    marker.index, marker.title, marker.start_ms, marker.end_ms
                );
            }
        }

        Some(Commands::Render {
            input,
            format,
            engine,
            voice,
            style,
            output_dir,
        }) => {
            let mut project = if input.to_string_lossy().ends_with(".oac.json") {
                BookProject::load_from_file(&input)?
            } else {
                BookProject::from_file(&input, CleanerOptions::default())?
            };

            project.export_config.output_dir = output_dir;
            project.export_config.format = match format.to_ascii_lowercase().as_str() {
                "mp3" => ExportFormat::Mp3Single,
                "mp3-chapters" => ExportFormat::Mp3PerChapter,
                "wav" => ExportFormat::Wav,
                _ => ExportFormat::M4b,
            };

            project.tts_config.engine = match engine.to_ascii_lowercase().as_str() {
                "piper" => TtsEngineKind::Piper,
                "xtts" | "coqui" => TtsEngineKind::CoquiXtts,
                "builtin" => TtsEngineKind::BuiltinAcoustic,
                _ => TtsEngineKind::Kokoro,
            };
            project.tts_config.voice_id = voice;
            project.tts_config.narration_style = match style.to_ascii_lowercase().as_str() {
                "professor" | "didatico" => NarrationStyle::Professor,
                "dramatic" | "dramatico" => NarrationStyle::Dramatic,
                "natural" => NarrationStyle::Natural,
                _ => NarrationStyle::Storyteller,
            };

            let artifact = render_audiobook(&mut project, Path::new(".oac-cache"), |prog| {
                println!(
                    "[{:3.0}%] {} ({}/{})",
                    prog.percent, prog.status, prog.completed_chunks, prog.total_chunks
                );
            })?;

            println!("✅ Audiobook finalizado: {}", artifact.primary_output_path);
        }

        Some(Commands::Voices { models_dir }) => {
            let diag = run_diagnostics();
            println!("🔍 Diagnóstico de Motores Locais:");
            println!(
                "   - Piper TTS:   {}",
                if diag.piper_installed {
                    "Instalado ✅"
                } else {
                    "Não detectado no PATH (usará fallback acústico)"
                }
            );
            println!(
                "   - Kokoro ONNX: {}",
                if diag.kokoro_installed {
                    "Instalado ✅"
                } else {
                    "Não detectado no PATH"
                }
            );
            println!(
                "   - Coqui XTTS:  {}",
                if diag.coqui_xtts_installed {
                    "Instalado ✅"
                } else {
                    "Não detectado no PATH"
                }
            );
            println!(
                "   - FFmpeg:      {}",
                if diag.ffmpeg_installed {
                    "Instalado ✅"
                } else {
                    "Não detectado (usará empacotador M4B/ID3 nativo em Rust)"
                }
            );
            println!("\n🎙️ Catálogo de Vozes Suportadas:");
            for v in list_voice_catalog(&models_dir) {
                println!(
                    "   • [{}] {} ({}) — {} | Modelo: {}",
                    v.id,
                    v.name,
                    v.language,
                    v.engine.display_name(),
                    v.model_filename
                );
            }
        }

        Some(Commands::Serve { port }) => {
            server::run_http_server(port, false).await?;
        }
    }

    Ok(())
}
