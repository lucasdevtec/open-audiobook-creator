use crate::error::{OacError, Result};
use crate::model::{TtsConfig, TtsEngineKind, VoiceProfile};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDiagnostics {
    pub piper_installed: bool,
    pub piper_binary: Option<String>,
    pub kokoro_installed: bool,
    pub kokoro_binary: Option<String>,
    pub coqui_xtts_installed: bool,
    pub coqui_binary: Option<String>,
    pub ffmpeg_installed: bool,
    pub ffmpeg_binary: Option<String>,
}

pub fn run_diagnostics() -> EngineDiagnostics {
    let piper_binary = find_binary(&["piper", "piper-tts"]);
    let kokoro_binary = find_binary(&["kokoro-tts", "kokoro"]);
    let coqui_binary = find_binary(&["tts", "xtts-server"]);
    let ffmpeg_binary = find_binary(&["ffmpeg"]);

    EngineDiagnostics {
        piper_installed: piper_binary.is_some(),
        piper_binary,
        kokoro_installed: kokoro_binary.is_some(),
        kokoro_binary,
        coqui_xtts_installed: coqui_binary.is_some(),
        coqui_binary,
        ffmpeg_installed: ffmpeg_binary.is_some(),
        ffmpeg_binary,
    }
}

pub fn resolve_resource_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(res_dir) = std::env::var("OAC_RESOURCE_DIR") {
        dirs.push(std::path::PathBuf::from(res_dir));
    }
    if let Ok(appdir) = std::env::var("APPDIR") {
        let appdir_path = std::path::PathBuf::from(appdir);
        dirs.push(appdir_path.join("usr/lib/open-audiobook-creator"));
        dirs.push(appdir_path.clone());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            dirs.push(exe_dir.to_path_buf());
            dirs.push(exe_dir.join("../lib/open-audiobook-creator"));
            dirs.push(exe_dir.join("../Resources"));
            if let Some(parent) = exe_dir.parent() {
                dirs.push(parent.to_path_buf());
                if let Some(grandparent) = parent.parent() {
                    dirs.push(grandparent.to_path_buf());
                }
            }
        }
    }
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        let appimage_path = std::path::PathBuf::from(appimage);
        if let Some(appimage_dir) = appimage_path.parent() {
            dirs.push(appimage_dir.to_path_buf());
            if let Some(parent) = appimage_dir.parent() {
                dirs.push(parent.to_path_buf());
            }
        }
    }
    if let Ok(owd) = std::env::var("OWD") {
        let owd_path = std::path::PathBuf::from(owd);
        dirs.push(owd_path.clone());
        if let Some(parent) = owd_path.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    dirs.push(std::path::PathBuf::from("."));
    dirs.push(std::path::PathBuf::from(".."));

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(std::path::PathBuf::from(home).join(".local/share/open-audiobook-creator"));
    }
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        dirs.push(std::path::PathBuf::from(local_appdata).join("open-audiobook-creator"));
    }
    dirs
}

pub fn resolve_model_file(configured_models_dir: &Path, filename: &str) -> Option<std::path::PathBuf> {
    let direct = configured_models_dir.join(filename);
    if direct.exists() {
        return Some(direct);
    }
    for base in resolve_resource_dirs() {
        let candidate = base.join("models").join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn find_binary(candidates: &[&str]) -> Option<String> {
    let local_subpaths = ["bin", "bin/piper", ".venv/bin", ".venv/Scripts"];
    let base_dirs = resolve_resource_dirs();

    for &bin in candidates {
        for base in &base_dirs {
            for sub in &local_subpaths {
                let local_candidate = base.join(sub).join(bin);
                if local_candidate.exists() {
                    return Some(local_candidate.to_string_lossy().to_string());
                }
                let exe_candidate = base.join(sub).join(format!("{bin}.exe"));
                if exe_candidate.exists() {
                    return Some(exe_candidate.to_string_lossy().to_string());
                }
            }
        }

        let ok = Command::new(bin)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .or_else(|_| {
                Command::new(bin)
                    .arg("--help")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            })
            .is_ok();
        if ok {
            return Some(bin.to_string());
        }
    }
    None
}

pub fn list_voice_catalog(models_dir: &Path) -> Vec<VoiceProfile> {
    let check_model = |filename: &str| resolve_model_file(models_dir, filename).is_some();
    let kokoro_ready = check_model("kokoro-v1_0.onnx") && check_model("voices-v1.0.bin");

    vec![
        // --- KOKORO-82M VOZES & BLENDS LITERÁRIOS EM PORTUGUÊS ---
        VoiceProfile {
            id: "kokoro-blend_cronista".to_string(),
            name: "O Cronista — Contador de Histórias (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino (Grave Literário)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Blend vocal profundo e sereno (Santa + George), ideal para fantasia épica, romances e crônicas.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-blend_bardo".to_string(),
            name: "Bardo da Pousada — Narrador Expressivo (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino (Caloroso)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Blend expressivo e dinâmico (Alex + Fable) para contos, diálogos e aventuras.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-blend_sombrio".to_string(),
            name: "Voz das Sombras — Suspense & Mistério (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino (Barítono Denso)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Timbre grave e cinematográfico (Onyx + Santa) para prólogos sombrios, terror e ficção.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-pm_santa".to_string(),
            name: "Kokoro Santa — Sábio & Acolhedor (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino (Maduro)".to_string(),
            quality: "Alta (24 kHz ONNX)".to_string(),
            description: "Voz nativa brasileira com timbre maduro, calmo e pausado para leitura clássica.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-blend_professor".to_string(),
            name: "Professor Mentor — Didático & Claro (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino (Didático)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Blend articulado (Alex + Daniel) com dicção limpa, ótimo para livros técnicos, história e estudo.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-blend_professora".to_string(),
            name: "Professora Clara — Acadêmica & Explicativa (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Feminino (Didático)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Blend feminino articulado (Dora + Emma) para artigos, ensaios, filosofia e aprendizado.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-blend_contadora".to_string(),
            name: "Contadora de Histórias — Suave & Emotiva (Português Brasil)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Feminino (Literário)".to_string(),
            quality: "Estúdio GPU/ONNX (24 kHz)".to_string(),
            description: "Blend feminino aveludado (Dora + Bella + Heart) para romances, poesia e literatura.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-pf_dora".to_string(),
            name: "Kokoro Dora (Português Brasil - Feminino)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Feminino".to_string(),
            quality: "Alta (24 kHz ONNX)".to_string(),
            description: "Voz nativa brasileira do modelo Kokoro-82M com entonação limpa.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-pm_alex".to_string(),
            name: "Kokoro Alex (Português Brasil - Masculino)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "pt-BR".to_string(),
            gender: "Masculino".to_string(),
            quality: "Alta (24 kHz ONNX)".to_string(),
            description: "Voz masculina nativa do modelo Kokoro-82M em Português.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        VoiceProfile {
            id: "kokoro-af_heart".to_string(),
            name: "Kokoro Heart (English US - Literary Narrator)".to_string(),
            engine: TtsEngineKind::Kokoro,
            language: "en-US".to_string(),
            gender: "Feminino".to_string(),
            quality: "Alta (24 kHz ONNX)".to_string(),
            description: "Voz premiada Kokoro-82M para narração literária em inglês.".to_string(),
            model_filename: "kokoro-v1_0.onnx".to_string(),
            sample_rate: 24000,
            installed: kokoro_ready,
        },
        // --- PIPER TTS VOZES ---
        VoiceProfile {
            id: "pt_BR-faber-medium".to_string(),
            name: "Piper Faber (Português Brasil - Masculino)".to_string(),
            engine: TtsEngineKind::Piper,
            language: "pt-BR".to_string(),
            gender: "Masculino".to_string(),
            quality: "Média (22.05 kHz)".to_string(),
            description: "Voz neural super rápida em CPU para leitura direta de livros em Português.".to_string(),
            model_filename: "pt_BR-faber-medium.onnx".to_string(),
            sample_rate: 22050,
            installed: check_model("pt_BR-faber-medium.onnx"),
        },
        VoiceProfile {
            id: "pt_BR-edresson-low".to_string(),
            name: "Piper Edresson (Português Brasil - Leve)".to_string(),
            engine: TtsEngineKind::Piper,
            language: "pt-BR".to_string(),
            gender: "Masculino".to_string(),
            quality: "Leve (16 kHz)".to_string(),
            description: "Voz ultra-rápida otimizada para CPUs modestas e livros longos.".to_string(),
            model_filename: "pt_BR-edresson-low.onnx".to_string(),
            sample_rate: 16000,
            installed: check_model("pt_BR-edresson-low.onnx"),
        },
        VoiceProfile {
            id: "en_US-lessac-high".to_string(),
            name: "Piper Lessac (English US - Studio)".to_string(),
            engine: TtsEngineKind::Piper,
            language: "en-US".to_string(),
            gender: "Feminino".to_string(),
            quality: "Alta (22.05 kHz)".to_string(),
            description: "Voz de estúdio expressiva para audiolivros em inglês.".to_string(),
            model_filename: "en_US-lessac-high.onnx".to_string(),
            sample_rate: 22050,
            installed: check_model("en_US-lessac-high.onnx"),
        },
        // --- COQUI XTTS & BUILTIN ---
        VoiceProfile {
            id: "xtts_v2_multilingual".to_string(),
            name: "Coqui XTTS-v2 (Clonagem de Voz Customizada)".to_string(),
            engine: TtsEngineKind::CoquiXtts,
            language: "pt-BR / Multi".to_string(),
            gender: "Customizável".to_string(),
            quality: "Estúdio (24 kHz)".to_string(),
            description: "Suporta clonagem de voz zero-shot via amostra WAV e prosódia rica.".to_string(),
            model_filename: "xtts_v2".to_string(),
            sample_rate: 24000,
            installed: find_binary(&["tts"]).is_some(),
        },
        VoiceProfile {
            id: "builtin-acoustic-pt".to_string(),
            name: "Cadência Acústica Integrada (Offline Instantâneo)".to_string(),
            engine: TtsEngineKind::BuiltinAcoustic,
            language: "Multi".to_string(),
            gender: "Neutro".to_string(),
            quality: "Preview (22.05 kHz)".to_string(),
            description: "Gerador acústico puro em Rust para validação de capítulos e testes sem modelos externos.".to_string(),
            model_filename: "builtin".to_string(),
            sample_rate: 22050,
            installed: true,
        },
    ]
}

/// Sintetiza um trecho de texto em arquivo WAV (`output_wav`) usando o motor configurado.
/// Retorna o nome descritivo do motor efetivamente utilizado.
pub fn synthesize_chunk_to_wav(
    raw_text: &str,
    config: &TtsConfig,
    output_wav: &Path,
) -> Result<String> {
    if let Some(parent) = output_wav.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Garante que mesmo textos brutos colados diretamente no Preview passem pela reparação literária
    let (cleaned_for_tts, _) = crate::cleaner::repair_literary_pdf_artifacts(raw_text);
    let text = cleaned_for_tts.as_str();

    match config.engine {
        TtsEngineKind::Piper => {
            match try_synthesize_piper(text, config, output_wav) {
                Ok(()) => Ok(format!(
                    "Piper TTS ({} • {})",
                    config.voice_id,
                    config.narration_style.display_name()
                )),
                Err(err) if config.fallback_to_builtin => {
                    synthesize_builtin_acoustic_wav(text, config, output_wav)?;
                    Ok(format!(
                        "Builtin Acoustic Fallback (Piper ausente: {})",
                        err
                    ))
                }
                Err(err) => Err(err),
            }
        }
        TtsEngineKind::Kokoro => {
            match try_synthesize_kokoro(text, config, output_wav) {
                Ok(()) => Ok(format!(
                    "Kokoro-82M GPU/ONNX ({} • {})",
                    config.voice_id,
                    config.narration_style.display_name()
                )),
                Err(err) if config.fallback_to_builtin => {
                    synthesize_builtin_acoustic_wav(text, config, output_wav)?;
                    Ok(format!(
                        "Builtin Acoustic Fallback (Kokoro ausente: {})",
                        err
                    ))
                }
                Err(err) => Err(err),
            }
        }
        TtsEngineKind::CoquiXtts => {
            match try_synthesize_coqui(text, config, output_wav) {
                Ok(()) => Ok(format!("Coqui XTTS-v2 ({})", config.voice_id)),
                Err(err) if config.fallback_to_builtin => {
                    synthesize_builtin_acoustic_wav(text, config, output_wav)?;
                    Ok(format!(
                        "Builtin Acoustic Fallback (XTTS ausente: {})",
                        err
                    ))
                }
                Err(err) => Err(err),
            }
        }
        TtsEngineKind::BuiltinAcoustic => {
            synthesize_builtin_acoustic_wav(text, config, output_wav)?;
            Ok("Sintetizador Acústico Integrado (Rust)".to_string())
        }
    }
}

fn try_synthesize_piper(text: &str, config: &TtsConfig, output_wav: &Path) -> Result<()> {
    let bin = config
        .binary_path
        .clone()
        .or_else(|| find_binary(&["piper", "piper-tts"]))
        .ok_or_else(|| OacError::TtsError {
            engine: "Piper".to_string(),
            message: "Binário 'piper' não encontrado no PATH.".to_string(),
        })?;

    let model_path = resolve_model_file(
        Path::new(&config.models_dir),
        &format!("{}.onnx", config.voice_id),
    )
    .ok_or_else(|| OacError::TtsError {
        engine: "Piper".to_string(),
        message: format!("Modelo ONNX '{}.onnx' não encontrado.", config.voice_id),
    })?;

    let style_speed_factor = match config.narration_style {
        crate::model::NarrationStyle::Storyteller => 0.93,
        crate::model::NarrationStyle::Professor => 0.96,
        crate::model::NarrationStyle::Dramatic => 0.89,
        crate::model::NarrationStyle::Natural => 1.0,
    };
    let effective_speed = (config.speed * style_speed_factor).clamp(0.5, 2.5);
    let length_scale = (1.0 / effective_speed).to_string();
    let sentence_silence = match config.narration_style {
        crate::model::NarrationStyle::Storyteller => "0.48",
        crate::model::NarrationStyle::Professor => "0.40",
        crate::model::NarrationStyle::Dramatic => "0.62",
        crate::model::NarrationStyle::Natural => "0.30",
    };

    let mut child = Command::new(&bin)
        .arg("--model")
        .arg(&model_path)
        .arg("--output_file")
        .arg(output_wav)
        .arg("--length_scale")
        .arg(&length_scale)
        .arg("--sentence_silence")
        .arg(sentence_silence)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OacError::TtsError {
            engine: "Piper".to_string(),
            message: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(OacError::TtsError {
            engine: "Piper".to_string(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

fn try_synthesize_kokoro(text: &str, config: &TtsConfig, output_wav: &Path) -> Result<()> {
    let bin = config
        .binary_path
        .clone()
        .or_else(|| find_binary(&["kokoro-tts", "kokoro"]))
        .ok_or_else(|| OacError::TtsError {
            engine: "Kokoro".to_string(),
            message: "Binário 'kokoro-tts' não encontrado no PATH.".to_string(),
        })?;

    let voice_code = config
        .voice_id
        .strip_prefix("kokoro-")
        .unwrap_or(&config.voice_id);

    let mut cmd = Command::new(&bin);
    cmd.arg("--voice")
        .arg(voice_code)
        .arg("--speed")
        .arg(config.speed.to_string())
        .arg("--style")
        .arg(config.narration_style.as_str())
        .arg("--output")
        .arg(output_wav);

    if let Some(model_file) = resolve_model_file(Path::new(&config.models_dir), "kokoro-v1_0.onnx")
    {
        if let Some(models_dir) = model_file.parent() {
            cmd.arg("--models-dir").arg(models_dir);
        }
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OacError::TtsError {
            engine: "Kokoro".to_string(),
            message: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(OacError::TtsError {
            engine: "Kokoro".to_string(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

fn try_synthesize_coqui(text: &str, config: &TtsConfig, output_wav: &Path) -> Result<()> {
    let bin = config
        .binary_path
        .clone()
        .or_else(|| find_binary(&["tts"]))
        .ok_or_else(|| OacError::TtsError {
            engine: "Coqui XTTS".to_string(),
            message: "CLI 'tts' (Coqui) não encontrado no PATH.".to_string(),
        })?;

    let mut cmd = Command::new(&bin);
    cmd.arg("--model_name")
        .arg("tts_models/multilingual/multi-dataset/xtts_v2")
        .arg("--text")
        .arg(text)
        .arg("--language_idx")
        .arg("pt")
        .arg("--out_path")
        .arg(output_wav);

    if let Some(ref speaker_wav) = config.xtts_speaker_wav {
        if !speaker_wav.trim().is_empty() {
            cmd.arg("--speaker_wav").arg(speaker_wav);
        }
    }

    let output = cmd.output().map_err(|e| OacError::TtsError {
        engine: "Coqui XTTS".to_string(),
        message: e.to_string(),
    })?;

    if !output.status.success() {
        return Err(OacError::TtsError {
            engine: "Coqui XTTS".to_string(),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Sintetizador acústico prosódico em Rust puro: gera um arquivo WAV válido
/// com modulação silábica suave proporcional às palavras e pontuação do texto.
pub fn synthesize_builtin_acoustic_wav(
    text: &str,
    config: &TtsConfig,
    output_wav: &Path,
) -> Result<()> {
    let sample_rate = 22050u32;
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(output_wav, spec)
        .map_err(|e| OacError::AudioError(format!("Falha ao criar WAV: {e}")))?;

    let speed = config.speed.clamp(0.5, 2.5);
    let words: Vec<&str> = text.split_whitespace().collect();

    // Base fundamental frequency (warm baritone/alto formant ~145 Hz - 195 Hz)
    let base_f0 = if config.voice_id.contains("dora")
        || config.voice_id.contains("heart")
        || config.voice_id.contains("lessac")
    {
        195.0f32
    } else {
        138.0f32
    };

    let mut phase = 0.0f32;
    let two_pi = std::f32::consts::PI * 2.0;

    for (w_idx, word) in words.iter().enumerate() {
        let syllables = (word.chars().count() / 3).clamp(1, 5);
        for s_idx in 0..syllables {
            let syl_duration_secs = (0.095 / speed).clamp(0.035, 0.18);
            let num_samples = (syl_duration_secs * sample_rate as f32) as usize;

            let char_seed = word.bytes().nth(s_idx).unwrap_or(b'a') as f32;
            let pitch_contour = 1.0 - (w_idx as f32 / (words.len().max(1) as f32)) * 0.12;
            let freq = base_f0 * pitch_contour + ((char_seed % 17.0) - 8.0) * 1.8;

            for i in 0..num_samples {
                let t = i as f32 / num_samples.max(1) as f32;
                // Smooth raised-cosine envelope per syllable
                let envelope = (t * std::f32::consts::PI).sin().powi(2);

                phase += two_pi * freq / sample_rate as f32;
                if phase > two_pi {
                    phase -= two_pi;
                }

                // Soft harmonic blend (fundamental + 2nd + 3rd formant warmth)
                let wave =
                    0.65 * phase.sin() + 0.25 * (2.0 * phase).sin() + 0.10 * (3.0 * phase).sin();
                let sample = (wave * envelope * 6500.0) as i16;
                writer
                    .write_sample(sample)
                    .map_err(|e| OacError::AudioError(e.to_string()))?;
            }

            // Brief inter-syllable micro-pause (12 ms)
            let gap_samples = (0.012 * sample_rate as f32 / speed) as usize;
            for _ in 0..gap_samples {
                writer
                    .write_sample(0i16)
                    .map_err(|e| OacError::AudioError(e.to_string()))?;
            }
        }

        // Inter-word or punctuation pause
        let ends_punct = word.ends_with(['.', '!', '?', ';', ':']);
        let ends_comma = word.ends_with(',');
        let pause_secs = if ends_punct {
            (config.sentence_pause_ms as f32 / 1000.0) / speed
        } else if ends_comma {
            0.14 / speed
        } else {
            0.038 / speed
        };

        let pause_samples = (pause_secs.clamp(0.02, 0.8) * sample_rate as f32) as usize;
        for _ in 0..pause_samples {
            writer
                .write_sample(0i16)
                .map_err(|e| OacError::AudioError(e.to_string()))?;
        }
    }

    writer
        .finalize()
        .map_err(|e| OacError::AudioError(e.to_string()))?;
    Ok(())
}

