use crate::error::{OacError, Result};
use crate::model::{BookMetadata, SourceFormat};
use crate::parser::{ExtractedDocument, RawPage};
use regex::Regex;
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub fn extract_epub_bytes(bytes: &[u8]) -> Result<ExtractedDocument> {
    let reader = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| OacError::EpubError(format!("ZIP inválido: {e}")))?;

    let mut metadata = BookMetadata::default();

    // Find OPF file inside archive
    let mut opf_path = None;
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            if name.ends_with(".opf") {
                opf_path = Some(name);
                break;
            }
        }
    }

    let mut html_files = Vec::new();
    if let Some(ref opf) = opf_path {
        let mut opf_content = String::new();
        if let Ok(mut file) = archive.by_name(opf) {
            let _ = file.read_to_string(&mut opf_content);
        }
        if let Some(title) = extract_xml_tag(&opf_content, "dc:title")
            .or_else(|| extract_xml_tag(&opf_content, "title"))
        {
            if !title.trim().is_empty() {
                metadata.title = decode_html_entities(title.trim());
            }
        }
        if let Some(creator) = extract_xml_tag(&opf_content, "dc:creator")
            .or_else(|| extract_xml_tag(&opf_content, "creator"))
        {
            if !creator.trim().is_empty() {
                metadata.author = decode_html_entities(creator.trim());
            }
        }
        if let Some(lang) = extract_xml_tag(&opf_content, "dc:language") {
            if !lang.trim().is_empty() {
                metadata.language = lang.trim().to_string();
            }
        }
    }

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            let lower = name.to_ascii_lowercase();
            if (lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm"))
                && !lower.contains("nav")
                && !lower.contains("toc")
            {
                html_files.push(name);
            }
        }
    }

    html_files.sort();

    let mut pages = Vec::new();
    for (idx, file_name) in html_files.iter().enumerate() {
        let mut raw_html = String::new();
        if let Ok(mut file) = archive.by_name(file_name) {
            let _ = file.read_to_string(&mut raw_html);
        }
        let heading = extract_first_heading(&raw_html);
        let plain_text = strip_html_tags(&raw_html);
        if !plain_text.trim().is_empty() {
            pages.push(RawPage {
                page_number: (idx + 1) as u32,
                chapter_hint: heading,
                text: plain_text,
            });
        }
    }

    if pages.is_empty() {
        return Err(OacError::EpubError(
            "Nenhum capítulo XHTML/HTML encontrado no arquivo EPUB.".to_string(),
        ));
    }

    Ok(ExtractedDocument {
        format: SourceFormat::Epub,
        metadata,
        pages,
    })
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>");
    let re = Regex::new(&pattern).ok()?;
    let caps = re.captures(xml)?;
    Some(caps.get(1)?.as_str().to_string())
}

fn extract_first_heading(html: &str) -> Option<String> {
    let re = Regex::new(r"(?is)<h[1-3][^>]*>(.*?)</h[1-3]>").ok()?;
    let caps = re.captures(html)?;
    let raw = strip_html_tags(caps.get(1)?.as_str());
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub fn strip_html_tags(html: &str) -> String {
    let script_re = Regex::new(r"(?is)<(script|style|head)[^>]*>.*?</(script|style|head)>").unwrap();
    let without_scripts = script_re.replace_all(html, "");

    let block_re = Regex::new(r"(?i)</?(p|div|h[1-6]|li|br|tr|blockquote|section|article)[^>]*>").unwrap();
    let with_newlines = block_re.replace_all(&without_scripts, "\n");

    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    let stripped = tag_re.replace_all(&with_newlines, "");

    decode_html_entities(&stripped)
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
}

