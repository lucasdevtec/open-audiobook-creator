export type SourceFormat = 'pdf' | 'epub' | 'txt';

export interface BookMetadata {
  title: string;
  author: string;
  narrator: string;
  language: string;
  year: string | null;
  description: string | null;
  publisher: string | null;
  cover_image_base64: string | null;
  cover_mime_type: string | null;
}

export interface CleanerOptions {
  remove_repeated_headers_footers: boolean;
  remove_page_numbers: boolean;
  fix_hyphenation: boolean;
  normalize_whitespace: boolean;
  max_chunk_chars: number;
  custom_ignore_regex: string | null;
}

export interface TextChunk {
  id: string;
  index: number;
  text: string;
  estimated_duration_secs: number;
  audio_cache_path: string | null;
  status: 'pending' | 'synthesizing' | 'completed' | 'failed';
}

export interface Chapter {
  id: string;
  index: number;
  title: string;
  enabled: boolean;
  start_page: number | null;
  end_page: number | null;
  raw_text: string;
  cleaned_text: string;
  chunks: TextChunk[];
}

export type TtsEngineKind = 'piper' | 'kokoro' | 'coqui_xtts' | 'builtin_acoustic';

export type NarrationStyle = 'storyteller' | 'professor' | 'dramatic' | 'natural';

export interface VoiceProfile {
  id: string;
  name: string;
  engine: TtsEngineKind;
  language: string;
  gender: string;
  quality: string;
  description: string;
  model_filename: string;
  sample_rate: number;
  installed: boolean;
}

export interface TtsConfig {
  engine: TtsEngineKind;
  voice_id: string;
  narration_style: NarrationStyle;
  speed: number;
  sentence_pause_ms: number;
  paragraph_pause_ms: number;
  binary_path: string | null;
  models_dir: string;
  xtts_server_url: string | null;
  xtts_speaker_wav: string | null;
  fallback_to_builtin: boolean;
}

export type ExportFormat = 'm4b' | 'mp3_single' | 'mp3_per_chapter' | 'wav';

export interface ExportConfig {
  format: ExportFormat;
  output_dir: string;
  bitrate_kbps: number;
  sample_rate: number;
  include_chapter_markers: boolean;
  embed_cover: boolean;
}

export interface CleaningReport {
  total_pages_processed: number;
  removed_headers_footers: string[];
  page_numbers_removed: number;
  hyphenations_fixed: number;
  chapters_detected: number;
  total_words: number;
  total_chunks: number;
  estimated_audio_duration_secs: number;
}

export interface BookProject {
  id: string;
  source_path: string | null;
  source_format: SourceFormat;
  metadata: BookMetadata;
  cleaner_options: CleanerOptions;
  cleaning_report: CleaningReport;
  chapters: Chapter[];
  tts_config: TtsConfig;
  export_config: ExportConfig;
  created_at: string;
  updated_at: string;
}

export interface EngineDiagnostics {
  piper_installed: boolean;
  piper_binary: string | null;
  kokoro_installed: boolean;
  kokoro_binary: string | null;
  coqui_xtts_installed: boolean;
  coqui_binary: string | null;
  ffmpeg_installed: boolean;
  ffmpeg_binary: string | null;
}

export interface SystemInfoResponse {
  diagnostics: EngineDiagnostics;
  voices: VoiceProfile[];
}

export interface ChapterMarker {
  index: number;
  title: string;
  start_ms: number;
  end_ms: number;
}

export interface RenderArtifact {
  project_id: string;
  format: ExportFormat;
  primary_output_path: string;
  chapter_files: string[];
  chapter_markers: ChapterMarker[];
  total_duration_secs: number;
  used_engine: string;
  encoder_note: string;
}

export interface VoicePreviewResponse {
  audio_base64: string;
  mime_type: string;
  used_engine: string;
}

