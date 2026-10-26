pub mod epub;
pub mod pdf;
pub mod txt;

use crate::error::{OacError, Result};
use crate::model::{BookMetadata, SourceFormat};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RawPage {
    pub page_number: u32,
    pub chapter_hint: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedDocument {
    pub format: SourceFormat,
    pub metadata: BookMetadata,
    pub pages: Vec<RawPage>,
}

pub fn extract_from_path(path: &Path) -> Result<ExtractedDocument> {
    let format = SourceFormat::from_path(path).ok_or_else(|| {
        OacError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("desconhecido")
                .to_string(),
        )
    })?;

    let bytes = std::fs::read(path)?;
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Livro Importado")
        .replace(['_', '-'], " ");

    extract_from_bytes(&bytes, format, Some(&fallback_title))
}

pub fn extract_from_bytes(
    bytes: &[u8],
    format: SourceFormat,
    fallback_title: Option<&str>,
) -> Result<ExtractedDocument> {
    let mut doc = match format {
        SourceFormat::Pdf => pdf::extract_pdf_bytes(bytes)?,
        SourceFormat::Epub => epub::extract_epub_bytes(bytes)?,
        SourceFormat::Txt => txt::extract_txt_bytes(bytes)?,
    };

    if (doc.metadata.title.trim().is_empty() || doc.metadata.title == "Livro Sem Título")
        && fallback_title.is_some()
    {
        if let Some(title) = fallback_title {
            doc.metadata.title = title.to_string();
        }
    }

    Ok(doc)
}

