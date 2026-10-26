use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Pdf,
    Epub,
    Txt,
}

impl SourceFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "pdf" => Some(Self::Pdf),
            "epub" => Some(Self::Epub),
            "txt" | "md" | "markdown" => Some(Self::Txt),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadata {
    pub title: String,
    pub author: String,
    pub narrator: String,
    pub language: String,
    pub year: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub cover_image_base64: Option<String>,
    pub cover_mime_type: Option<String>,
}

impl Default for BookMetadata {
    fn default() -> Self {
        Self {
            title: "Livro Sem Título".to_string(),
            author: "Autor Desconhecido".to_string(),
            narrator: "Open Audiobook Creator (TTS)".to_string(),
            language: "pt-BR".to_string(),
            year: Some("2026".to_string()),
            description: None,
            publisher: None,
            cover_image_base64: None,
            cover_mime_type: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanerOptions {
    pub remove_repeated_headers_footers: bool,
    pub remove_page_numbers: bool,
    pub fix_hyphenation: bool,
    pub normalize_whitespace: bool,
    pub max_chunk_chars: usize,
    pub custom_ignore_regex: Option<String>,
}

impl Default for CleanerOptions {
    fn default() -> Self {
        Self {
            remove_repeated_headers_footers: true,
            remove_page_numbers: true,
            fix_hyphenation: true,
            normalize_whitespace: true,
            max_chunk_chars: 260,
            custom_ignore_regex: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkStatus {
    Pending,
    Synthesizing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    pub id: String,
    pub index: usize,
    pub text: String,
    pub estimated_duration_secs: f32,
    pub audio_cache_path: Option<String>,
    pub status: ChunkStatus,
}

impl TextChunk {
    pub fn new(index: usize, text: String) -> Self {
        // Estimate ~14 chars per second of spoken Portuguese/English audio
        let estimated_duration_secs = (text.chars().count() as f32 / 14.0).max(0.8);
        Self {
            id: Uuid::new_v4().to_string(),
            index,
            text,
            estimated_duration_secs,
            audio_cache_path: None,
            status: ChunkStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub index: usize,
    pub title: String,
    pub enabled: bool,
    pub start_page: Option<u32>,
    pub end_page: Option<u32>,
    pub raw_text: String,
    pub cleaned_text: String,
    pub chunks: Vec<TextChunk>,
}

impl Chapter {
    pub fn total_estimated_duration_secs(&self) -> f32 {
        self.chunks.iter().map(|c| c.estimated_duration_secs).sum()
    }

    pub fn word_count(&self) -> usize {
        self.cleaned_text.split_whitespace().count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsEngineKind {
    Piper,
    Kokoro,
    CoquiXtts,
    BuiltinAcoustic,
}

impl TtsEngineKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Piper => "Piper TTS (Neural Local / ONNX)",
            Self::Kokoro => "Kokoro-82M (Ultra-Natural ONNX)",
            Self::CoquiXtts => "Coqui XTTS-v2 (Multilíngue / Voice Clone)",
            Self::BuiltinAcoustic => "Sintetizador Acústico Integrado (Preview Offline)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NarrationStyle {
    #[default]
    Storyteller,
    Professor,
    Dramatic,
    Natural,
}

impl NarrationStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Storyteller => "storyteller",
            Self::Professor => "professor",
            Self::Dramatic => "dramatic",
            Self::Natural => "natural",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Storyteller => "Contador de Histórias (Literário / Envolvente)",
            Self::Professor => "Professor / Didático (Articulado e Explicativo)",
            Self::Dramatic => "Dramático / Épico (Pausas Densas & Grave)",
            Self::Natural => "Leitura Natural (Equilibrado)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub engine: TtsEngineKind,
    pub language: String,
    pub gender: String,
    pub quality: String,
    pub description: String,
    pub model_filename: String,
    pub sample_rate: u32,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub engine: TtsEngineKind,
    pub voice_id: String,
    #[serde(default)]
    pub narration_style: NarrationStyle,
    pub speed: f32,
    pub sentence_pause_ms: u32,
    pub paragraph_pause_ms: u32,
    pub binary_path: Option<String>,
    pub models_dir: String,
    pub xtts_server_url: Option<String>,
    pub xtts_speaker_wav: Option<String>,
    pub fallback_to_builtin: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: TtsEngineKind::Kokoro,
            voice_id: "kokoro-blend_cronista".to_string(),
            narration_style: NarrationStyle::Storyteller,
            speed: 0.94,
            sentence_pause_ms: 420,
            paragraph_pause_ms: 850,
            binary_path: None,
            models_dir: "./models".to_string(),
            xtts_server_url: Some("http://127.0.0.1:8020".to_string()),
            xtts_speaker_wav: None,
            fallback_to_builtin: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    M4b,
    Mp3Single,
    Mp3PerChapter,
    Wav,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub output_dir: String,
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub include_chapter_markers: bool,
    pub embed_cover: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::M4b,
            output_dir: "./output".to_string(),
            bitrate_kbps: 128,
            sample_rate: 22050,
            include_chapter_markers: true,
            embed_cover: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningReport {
    pub total_pages_processed: usize,
    pub removed_headers_footers: Vec<String>,
    pub page_numbers_removed: usize,
    pub hyphenations_fixed: usize,
    pub chapters_detected: usize,
    pub total_words: usize,
    pub total_chunks: usize,
    pub estimated_audio_duration_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookProject {
    pub id: String,
    pub source_path: Option<String>,
    pub source_format: SourceFormat,
    pub metadata: BookMetadata,
    pub cleaner_options: CleanerOptions,
    pub cleaning_report: CleaningReport,
    pub chapters: Vec<Chapter>,
    pub tts_config: TtsConfig,
    pub export_config: ExportConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMarker {
    pub index: usize,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderArtifact {
    pub project_id: String,
    pub format: ExportFormat,
    pub primary_output_path: String,
    pub chapter_files: Vec<String>,
    pub chapter_markers: Vec<ChapterMarker>,
    pub total_duration_secs: f32,
    pub used_engine: String,
    pub encoder_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderProgress {
    pub project_id: String,
    pub status: String,
    pub current_chapter_index: usize,
    pub total_chapters: usize,
    pub current_chapter_title: String,
    pub completed_chunks: usize,
    pub total_chunks: usize,
    pub percent: f32,
    pub finished: bool,
    pub error: Option<String>,
    pub artifact: Option<RenderArtifact>,
}

