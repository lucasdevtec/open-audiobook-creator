use crate::error::Result;
use crate::model::{BookMetadata, SourceFormat};
use crate::parser::{ExtractedDocument, RawPage};

pub fn extract_txt_bytes(bytes: &[u8]) -> Result<ExtractedDocument> {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };

    let metadata = BookMetadata::default();

    // Split text into virtual pages if form-feed (\x0C) is present, otherwise treat as pages of ~45 lines
    let raw_pages: Vec<String> = if text.contains('\x0C') {
        text.split('\x0C').map(|s| s.to_string()).collect()
    } else {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            vec![text]
        } else {
            lines
                .chunks(45)
                .map(|chunk| chunk.join("\n"))
                .collect()
        }
    };

    let pages = raw_pages
        .into_iter()
        .enumerate()
        .map(|(idx, p)| RawPage {
            page_number: (idx + 1) as u32,
            chapter_hint: None,
            text: p,
        })
        .collect();

    Ok(ExtractedDocument {
        format: SourceFormat::Txt,
        metadata,
        pages,
    })
}

