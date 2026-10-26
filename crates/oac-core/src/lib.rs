pub mod audio;
pub mod cleaner;
pub mod dialogue;
pub mod error;
pub mod mastering;
pub mod model;
pub mod parser;
pub mod text_normalizer;
pub mod tts;

use crate::cleaner::{clean_chapter_text, process_pages_into_chapters, split_text_into_chunks};
use crate::error::{OacError, Result};
use crate::model::{
    BookProject, Chapter, CleanerOptions, ExportConfig, SourceFormat, TtsConfig,
};
use crate::parser::{extract_from_bytes, extract_from_path};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

impl BookProject {
    pub fn from_file(path: &Path, cleaner_options: CleanerOptions) -> Result<Self> {
        let extracted = extract_from_path(path)?;
        let processed = process_pages_into_chapters(&extracted.pages, &cleaner_options);
        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            source_path: Some(path.to_string_lossy().to_string()),
            source_format: extracted.format,
            metadata: extracted.metadata,
            cleaner_options,
            cleaning_report: processed.report,
            chapters: processed.chapters,
            tts_config: TtsConfig::default(),
            export_config: ExportConfig::default(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn from_bytes(
        bytes: &[u8],
        format: SourceFormat,
        fallback_title: Option<&str>,
        cleaner_options: CleanerOptions,
    ) -> Result<Self> {
        let extracted = extract_from_bytes(bytes, format, fallback_title)?;
        let processed = process_pages_into_chapters(&extracted.pages, &cleaner_options);
        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            source_path: None,
            source_format: extracted.format,
            metadata: extracted.metadata,
            cleaner_options,
            cleaning_report: processed.report,
            chapters: processed.chapters,
            tts_config: TtsConfig::default(),
            export_config: ExportConfig::default(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn from_sample_pdf() -> Result<Self> {
        let pdf_bytes = parser::pdf::generate_sample_pdf_bytes()?;
        Self::from_bytes(
            &pdf_bytes,
            SourceFormat::Pdf,
            Some("A Arquitetura do Silêncio Digital"),
            CleanerOptions::default(),
        )
    }

    pub fn update_chapter(
        &mut self,
        chapter_id: &str,
        new_title: Option<String>,
        new_cleaned_text: Option<String>,
        enabled: Option<bool>,
    ) -> Result<()> {
        let max_chars = self.cleaner_options.max_chunk_chars;
        let chapter = self
            .chapters
            .iter_mut()
            .find(|c| c.id == chapter_id)
            .ok_or_else(|| OacError::ChapterNotFound(chapter_id.to_string()))?;

        if let Some(title) = new_title {
            chapter.title = title;
        }
        if let Some(text) = new_cleaned_text {
            chapter.cleaned_text = text;
            chapter.chunks = split_text_into_chunks(&chapter.cleaned_text, max_chars);
        }
        if let Some(en) = enabled {
            chapter.enabled = en;
        }

        self.recompute_report_metrics();
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn re_apply_cleaner(&mut self, options: CleanerOptions) {
        self.cleaner_options = options.clone();
        let mut total_hyph = 0usize;
        for chapter in &mut self.chapters {
            let (cleaned, hyph) = clean_chapter_text(&chapter.raw_text, &options);
            total_hyph += hyph;
            chapter.cleaned_text = cleaned;
            chapter.chunks =
                split_text_into_chunks(&chapter.cleaned_text, options.max_chunk_chars);
        }
        self.cleaning_report.hyphenations_fixed = total_hyph;
        self.recompute_report_metrics();
        self.updated_at = Utc::now();
    }

    pub fn merge_chapter_with_next(&mut self, chapter_id: &str) -> Result<()> {
        let pos = self
            .chapters
            .iter()
            .position(|c| c.id == chapter_id)
            .ok_or_else(|| OacError::ChapterNotFound(chapter_id.to_string()))?;

        if pos + 1 >= self.chapters.len() {
            return Ok(());
        }

        let next = self.chapters.remove(pos + 1);
        let current = &mut self.chapters[pos];
        current.raw_text = format!("{}\n\n{}", current.raw_text, next.raw_text);
        current.cleaned_text = format!("{}\n\n{}", current.cleaned_text, next.cleaned_text);
        if next.end_page.is_some() {
            current.end_page = next.end_page;
        }
        current.chunks = split_text_into_chunks(
            &current.cleaned_text,
            self.cleaner_options.max_chunk_chars,
        );

        for (i, chap) in self.chapters.iter_mut().enumerate() {
            chap.index = i + 1;
        }
        self.recompute_report_metrics();
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn add_custom_chapter(&mut self, title: String, text: String) {
        let idx = self.chapters.len() + 1;
        let (cleaned_text, _) = clean_chapter_text(&text, &self.cleaner_options);
        let chunks = split_text_into_chunks(&cleaned_text, self.cleaner_options.max_chunk_chars);
        self.chapters.push(Chapter {
            id: Uuid::new_v4().to_string(),
            index: idx,
            title,
            enabled: true,
            start_page: None,
            end_page: None,
            raw_text: text,
            cleaned_text,
            chunks,
        });
        self.recompute_report_metrics();
        self.updated_at = Utc::now();
    }

    pub fn recompute_report_metrics(&mut self) {
        self.cleaning_report.chapters_detected = self.chapters.len();
        self.cleaning_report.total_words = self
            .chapters
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.word_count())
            .sum();
        self.cleaning_report.total_chunks = self
            .chapters
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.chunks.len())
            .sum();
        self.cleaning_report.estimated_audio_duration_secs = self
            .chapters
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.total_estimated_duration_secs())
            .sum();
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let project: Self = serde_json::from_str(&content)?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::render_audiobook;
    use crate::model::{ExportFormat, TtsEngineKind};

    #[test]
    fn test_sample_pdf_extraction_and_smart_cleaning() {
        let project = BookProject::from_sample_pdf().expect("deve extrair o PDF de amostra");
        assert_eq!(project.metadata.title, "A Arquitetura do Silêncio Digital");
        assert_eq!(project.metadata.author, "Dra. Helena Vasconcelos");

        // Deve detectar os 3 capítulos do PDF de amostra
        assert_eq!(project.chapters.len(), 3);
        assert!(project.chapters[0].title.contains("Capítulo 1"));
        assert!(project.chapters[1].title.contains("Capítulo 2"));
        assert!(project.chapters[2].title.contains("Capítulo 3"));

        // Deve ter removido cabeçalhos repetidos e números de página
        assert!(project.cleaning_report.page_numbers_removed >= 4);
        assert!(!project.cleaning_report.removed_headers_footers.is_empty());

        // Deve ter corrigido palavras hifenizadas ("naturali-\ndade" -> "naturalidade")
        assert!(project.chapters[0].cleaned_text.contains("naturalidade"));
        assert!(!project.chapters[0].cleaned_text.contains("naturali-"));
    }

    #[test]
    fn test_literary_pdf_prologue_cleaning() {
        let raw = "Um silêncio de três partes N OITE OUTRA VEZ. A Pousada Marco do Percurso estava em si\u{FFFD}lêncio, e era um silêncio em três partes. Ele estava no peso da lareira de pedras negras, que conser vava o calor de um fogo há muito extinto. Seus olhos eram es curos e distantes.";
        let (cleaned, fixes) =
            crate::cleaner::clean_chapter_text(raw, &CleanerOptions::default());

        assert!(fixes >= 4);
        assert!(cleaned.contains("Um silêncio de três partes."));
        assert!(cleaned.contains("Noite outra vez."));
        assert!(!cleaned.contains("N OITE"));
        assert!(cleaned.contains("silêncio"));
        assert!(cleaned.contains("conservava"));
        assert!(cleaned.contains("escuros"));
    }

    #[test]
    fn test_chapter_editing_and_merging() {
        let mut project = BookProject::from_sample_pdf().unwrap();
        let first_id = project.chapters[0].id.clone();

        project
            .update_chapter(
                &first_id,
                Some("Prólogo Editado".to_string()),
                Some("Primeira frase revisada. Segunda frase para síntese.".to_string()),
                Some(true),
            )
            .unwrap();

        assert_eq!(project.chapters[0].title, "Prólogo Editado");
        assert!(!project.chapters[0].chunks.is_empty());

        project.merge_chapter_with_next(&first_id).unwrap();
        assert_eq!(project.chapters.len(), 2);
    }

    #[test]
    fn test_audiobook_render_m4b_and_mp3() {
        let mut project = BookProject::from_sample_pdf().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("oac_test_{}", Uuid::new_v4()));
        let cache_dir = temp_dir.join("cache");
        let out_dir = temp_dir.join("out");

        project.tts_config.engine = TtsEngineKind::BuiltinAcoustic;
        project.export_config.output_dir = out_dir.to_string_lossy().to_string();
        project.export_config.format = ExportFormat::M4b;

        let artifact = render_audiobook(&mut project, &cache_dir, |_prog| {}).unwrap();
        assert!(Path::new(&artifact.primary_output_path).exists());
        assert_eq!(artifact.chapter_markers.len(), 3);
        assert!(artifact.total_duration_secs > 0.5);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

