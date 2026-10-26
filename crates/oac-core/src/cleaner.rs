use crate::model::{Chapter, CleanerOptions, CleaningReport, TextChunk};
use crate::parser::RawPage;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct ProcessedBook {
    pub chapters: Vec<Chapter>,
    pub report: CleaningReport,
}

pub fn process_pages_into_chapters(
    pages: &[RawPage],
    options: &CleanerOptions,
) -> ProcessedBook {
    let total_pages_processed = pages.len();
    let mut repeated_headers_footers = HashSet::new();

    if options.remove_repeated_headers_footers && pages.len() >= 2 {
        let mut edge_counts: HashMap<String, usize> = HashMap::new();
        for page in pages {
            let non_empty: Vec<String> = page
                .text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !is_page_number_line(l))
                .collect();

            if let Some(first) = non_empty.first() {
                if first.chars().count() <= 90 && !is_chapter_heading(first) {
                    *edge_counts.entry(normalize_header_key(first)).or_insert(0) += 1;
                }
            }
            if non_empty.len() > 1 {
                if let Some(last) = non_empty.last() {
                    if last.chars().count() <= 90 && !is_chapter_heading(last) {
                        *edge_counts.entry(normalize_header_key(last)).or_insert(0) += 1;
                    }
                }
            }
        }

        let threshold = (pages.len() / 2).max(2);
        for (line_key, count) in edge_counts {
            if count >= threshold && !line_key.is_empty() {
                repeated_headers_footers.insert(line_key);
            }
        }
    }

    let custom_regex = options
        .custom_ignore_regex
        .as_ref()
        .and_then(|pat| if pat.trim().is_empty() { None } else { Regex::new(pat).ok() });

    let mut page_numbers_removed = 0usize;
    let mut hyphenations_fixed = 0usize;

    // Collect cleaned lines with page tracking for chapter splitting
    #[derive(Clone)]
    struct TaggedLine {
        page_num: u32,
        text: String,
        chapter_hint: Option<String>,
    }

    let mut tagged_lines: Vec<TaggedLine> = Vec::new();

    for page in pages {
        let raw_lines: Vec<&str> = page.text.lines().collect();
        let non_empty_indices: Vec<usize> = raw_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, _)| i)
            .collect();

        let first_idx = non_empty_indices.first().copied();
        let last_idx = non_empty_indices.last().copied();
        let mut first_emitted_on_page = true;

        for (idx, raw_line) in raw_lines.into_iter().enumerate() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                tagged_lines.push(TaggedLine {
                    page_num: page.page_number,
                    text: String::new(),
                    chapter_hint: None,
                });
                continue;
            }

            let is_edge = Some(idx) == first_idx || Some(idx) == last_idx;

            if options.remove_page_numbers && is_edge && is_page_number_line(trimmed) {
                page_numbers_removed += 1;
                continue;
            }

            if options.remove_repeated_headers_footers && is_edge {
                let key = normalize_header_key(trimmed);
                if repeated_headers_footers.contains(&key) {
                    continue;
                }
            }

            if let Some(ref re) = custom_regex {
                if re.is_match(trimmed) {
                    continue;
                }
            }

            let hint = if first_emitted_on_page {
                first_emitted_on_page = false;
                page.chapter_hint.clone()
            } else {
                None
            };

            tagged_lines.push(TaggedLine {
                page_num: page.page_number,
                text: trimmed.to_string(),
                chapter_hint: hint,
            });
        }
    }

    // Group lines into raw chapters by detecting chapter headings or EPUB chapter hints
    struct RawChapterDraft {
        title: String,
        start_page: u32,
        end_page: u32,
        lines: Vec<String>,
    }

    let mut drafts: Vec<RawChapterDraft> = Vec::new();

    for item in tagged_lines {
        if let Some(hint) = item.chapter_hint {
            if !drafts.is_empty() || !hint.trim().is_empty() {
                drafts.push(RawChapterDraft {
                    title: hint.trim().to_string(),
                    start_page: item.page_num,
                    end_page: item.page_num,
                    lines: vec![item.text],
                });
                continue;
            }
        }

        if is_chapter_heading(&item.text) {
            let clean_title = item.text.trim_start_matches('#').trim().to_string();
            drafts.push(RawChapterDraft {
                title: clean_title,
                start_page: item.page_num,
                end_page: item.page_num,
                lines: Vec::new(),
            });
            continue;
        }

        if let Some(current) = drafts.last_mut() {
            current.end_page = item.page_num;
            current.lines.push(item.text);
        } else if !item.text.is_empty() {
            drafts.push(RawChapterDraft {
                title: "Capítulo 1".to_string(),
                start_page: item.page_num,
                end_page: item.page_num,
                lines: vec![item.text],
            });
        }
    }

    // Remove empty initial drafts if a heading came right after a pre-title line
    let non_empty_drafts: Vec<RawChapterDraft> = drafts
        .into_iter()
        .filter(|d| d.lines.iter().any(|l| !l.trim().is_empty()))
        .collect();

    let mut chapters = Vec::new();
    for (idx, draft) in non_empty_drafts.into_iter().enumerate() {
        let raw_text = draft.lines.join("\n");
        let (cleaned_text, hyph_count) = clean_chapter_text(&raw_text, options);
        hyphenations_fixed += hyph_count;

        let chunks = split_text_into_chunks(&cleaned_text, options.max_chunk_chars);

        chapters.push(Chapter {
            id: Uuid::new_v4().to_string(),
            index: idx + 1,
            title: draft.title,
            enabled: true,
            start_page: Some(draft.start_page),
            end_page: Some(draft.end_page),
            raw_text,
            cleaned_text,
            chunks,
        });
    }

    let total_words: usize = chapters.iter().map(|c| c.word_count()).sum();
    let total_chunks: usize = chapters.iter().map(|c| c.chunks.len()).sum();
    let estimated_audio_duration_secs: f32 = chapters
        .iter()
        .map(|c| c.total_estimated_duration_secs())
        .sum();

    let mut removed_headers_list: Vec<String> = repeated_headers_footers.into_iter().collect();
    removed_headers_list.sort();

    let report = CleaningReport {
        total_pages_processed,
        removed_headers_footers: removed_headers_list,
        page_numbers_removed,
        hyphenations_fixed,
        chapters_detected: chapters.len(),
        total_words,
        total_chunks,
        estimated_audio_duration_secs,
    };

    ProcessedBook { chapters, report }
}

pub fn clean_chapter_text(raw_text: &str, options: &CleanerOptions) -> (String, usize) {
    let mut hyphenations_fixed = 0usize;
    let lines: Vec<&str> = raw_text.lines().collect();

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current_para = String::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_para.is_empty() {
                paragraphs.push(current_para.trim().to_string());
                current_para.clear();
            }
            continue;
        }

        // Se a primeira linha do capítulo for um subtítulo curto sem pontuação final
        // (ex: "Um silêncio de três partes"), separe-o com ponto final em seu próprio parágrafo
        if line_idx == 0
            && current_para.is_empty()
            && trimmed.chars().count() <= 55
            && !trimmed.ends_with(['.', '!', '?', ':', ';', ',', '-', '–', '—'])
            && lines.len() > 1
        {
            paragraphs.push(format!("{trimmed}."));
            continue;
        }

        if current_para.is_empty() {
            current_para.push_str(trimmed);
        } else if options.fix_hyphenation && is_hyphenated_break(&current_para, trimmed) {
            // Remove trailing hyphen / soft-hyphen / replacement char
            while current_para.ends_with(['-', '\u{00AD}', '\u{FFFD}', '–']) {
                current_para.pop();
            }
            current_para.push_str(trimmed);
            hyphenations_fixed += 1;
        } else if options.normalize_whitespace {
            current_para.push(' ');
            current_para.push_str(trimmed);
        } else {
            current_para.push('\n');
            current_para.push_str(trimmed);
        }
    }

    if !current_para.is_empty() {
        paragraphs.push(current_para.trim().to_string());
    }

    let mut result = paragraphs.join("\n\n");
    if options.fix_hyphenation {
        let (repaired, extra_fixes) = repair_literary_pdf_artifacts(&result);
        result = repaired;
        hyphenations_fixed += extra_fixes;
    }

    if options.normalize_whitespace {
        let multi_space = Regex::new(r"[ \t]{2,}").unwrap();
        result = multi_space.replace_all(&result, " ").to_string();
    }

    (result, hyphenations_fixed)
}

fn is_hyphenated_break(prev_para: &str, next_line: &str) -> bool {
    if !prev_para.ends_with(['-', '\u{00AD}', '\u{FFFD}']) {
        return false;
    }
    let trimmed_prev = prev_para.trim_end_matches(['-', '\u{00AD}', '\u{FFFD}']);
    let before_hyphen = trimmed_prev.chars().last();
    let first_next = next_line.chars().next();
    match (before_hyphen, first_next) {
        (Some(prev_ch), Some(next_ch)) => prev_ch.is_alphabetic() && next_ch.is_lowercase(),
        _ => false,
    }
}

/// Corrige artefatos típicos de livros literários em PDF:
/// 1. Caracteres de substituição `\u{FFFD}` ou soft-hyphen `\u{00AD}` no meio de palavras (`si\u{FFFD}lêncio` -> `silêncio`)
/// 2. Letras Capitulares (Drop Caps) separadas no início de seção (`N OITE OUTRA VEZ` -> `Noite outra vez`)
/// 3. Subtítulo colado antes de capitular (`Um silêncio de três partes NOITE` -> `Um silêncio de três partes.\n\nNoite`)
/// 4. Palavras quebradas com espaço onde o hífen do PDF sumiu (`conser vava` -> `conservava`, `es curos` -> `escuros`)
/// 5. Reticências e travessões literários para pausas naturais no TTS
pub fn repair_literary_pdf_artifacts(input: &str) -> (String, usize) {
    let mut fixes = 0usize;
    let mut text = input.to_string();

    // 1. Remover \u{FFFD} ou \u{00AD} quando estiver entre duas letras minúsculas (ex: "si\u{FFFD}lêncio" -> "silêncio")
    let mid_word_bad_char = Regex::new(r"([a-záéíóúâêôãõç])[\u{FFFD}\u{00AD}]+([a-záéíóúâêôãõç])").unwrap();
    let count_bad = mid_word_bad_char.find_iter(&text).count();
    if count_bad > 0 {
        text = mid_word_bad_char.replace_all(&text, "$1$2").to_string();
        fixes += count_bad;
    }

    // 2. Unir letras capitulares (Drop Caps) seguidas de resto da palavra em maiúsculas
    // Ex: "N OITE" -> "NOITE", "Q UANDO" -> "QUANDO", "D ENTRO" -> "DENTRO"
    let drop_cap_re = Regex::new(r"\b([B-DF-HJ-NP-TV-Z])\s+([A-ZÁÉÍÓÚÂÊÔÃÕÇ]{2,})\b").unwrap();
    let count_dc = drop_cap_re.find_iter(&text).count();
    if count_dc > 0 {
        text = drop_cap_re.replace_all(&text, "$1$2").to_string();
        fixes += count_dc;
    }

    // 3. Se houver um subtítulo sem ponto colado antes de abertura em CAIXA ALTA (ex: "três partes NOITE OUTRA VEZ.")
    let glued_subtitle_re = Regex::new(
        r"([a-záéíóúâêôãõç]{2,})\s+([A-ZÁÉÍÓÚÂÊÔÃÕÇ]{3,}(?:\s+[A-ZÁÉÍÓÚÂÊÔÃÕÇ]{2,})+)",
    )
    .unwrap();
    if glued_subtitle_re.is_match(&text) {
        text = glued_subtitle_re
            .replace_all(&text, "$1.\n\n$2")
            .to_string();
        fixes += 1;
    }

    // 4. Converter aberturas de capítulo em CAIXA ALTA (ex: "NOITE OUTRA VEZ.") para Capitalização normal ("Noite outra vez.")
    // evitando que o TTS soletre ou leia com entonação robótica de sigla
    let all_caps_lead_re =
        Regex::new(r"\b([A-ZÁÉÍÓÚÂÊÔÃÕÇ]{2,}(?:\s+[A-ZÁÉÍÓÚÂÊÔÃÕÇ]{2,})+)\.").unwrap();
    text = all_caps_lead_re
        .replace_all(&text, |caps: &regex::Captures| {
            let phrase = &caps[1];
            let lower: String = phrase.chars().flat_map(|c| c.to_lowercase()).collect();
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => {
                    let cap: String = first.to_uppercase().collect();
                    format!("{cap}{}.", chars.as_str())
                }
                None => format!("{phrase}."),
            }
        })
        .to_string();

    // 5. Reconstruir palavras quebradas por falha de hifenização do PDF (onde o hífen sumiu e virou espaço)
    // Sufixos verbais/nominais em português que nunca são palavras isoladas válidas:
    let broken_suffix_re = Regex::new(
        r"\b([a-záéíóúâêôãõç]{2,})\s+(vava|vavam|curos|curas|lêncio|burinho|brias|tícias|ciente|cientes|nhavam|raria|rariam|savam|tavam)\b",
    )
    .unwrap();
    let count_suf = broken_suffix_re.find_iter(&text).count();
    if count_suf > 0 {
        text = broken_suffix_re.replace_all(&text, "$1$2").to_string();
        fixes += count_suf;
    }

    // 6. Normalizar travessões e reticências espaçadas para pausas dramáticas naturais no TTS
    let spaced_ellipsis_re = Regex::new(r"\s*…\s*").unwrap();
    text = spaced_ellipsis_re.replace_all(&text, "... ").to_string();

    (text, fixes)
}

pub fn split_text_into_chunks(text: &str, max_chars: usize) -> Vec<TextChunk> {
    let limit = max_chars.max(80);
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n") {
        let para = paragraph.trim();
        if para.is_empty() {
            continue;
        }

        let sentences = split_sentences(para);
        for sentence in sentences {
            let s = sentence.trim();
            if s.is_empty() {
                continue;
            }

            if current.is_empty() {
                if s.chars().count() <= limit {
                    current.push_str(s);
                } else {
                    for sub in split_long_sentence(s, limit) {
                        let idx = chunks.len();
                        chunks.push(TextChunk::new(idx, sub));
                    }
                }
            } else if current.chars().count() + 1 + s.chars().count() <= limit {
                current.push(' ');
                current.push_str(s);
            } else {
                let idx = chunks.len();
                chunks.push(TextChunk::new(idx, current.clone()));
                current.clear();

                if s.chars().count() <= limit {
                    current.push_str(s);
                } else {
                    for sub in split_long_sentence(s, limit) {
                        let sub_idx = chunks.len();
                        chunks.push(TextChunk::new(sub_idx, sub));
                    }
                }
            }
        }

        if !current.is_empty() {
            let idx = chunks.len();
            chunks.push(TextChunk::new(idx, current.clone()));
            current.clear();
        }
    }

    if !current.is_empty() {
        let idx = chunks.len();
        chunks.push(TextChunk::new(idx, current));
    }

    chunks
}

fn split_sentences(paragraph: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut buf = String::new();

    let chars: Vec<char> = paragraph.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        buf.push(ch);
        if matches!(ch, '.' | '!' | '?' | ';') {
            let next_is_space_or_end = chars
                .get(i + 1)
                .map(|c| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | '”'))
                .unwrap_or(true);
            if next_is_space_or_end && buf.trim().chars().count() > 8 {
                sentences.push(buf.trim().to_string());
                buf.clear();
            }
        }
    }
    if !buf.trim().is_empty() {
        sentences.push(buf.trim().to_string());
    }
    sentences
}

fn split_long_sentence(sentence: &str, max_chars: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for word in sentence.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            parts.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

pub fn is_page_number_line(line: &str) -> bool {
    let trimmed = line.trim();
    let re = Regex::new(
        r"(?i)^(?:p[áa]gina|page|p\.?|pg\.?)?\s*[-–—\[]?\s*\d{1,5}(?:\s*(?:/|de|of)\s*\d{1,5})?\s*[-–—\]]?$",
    )
    .unwrap();
    re.is_match(trimmed)
}

pub fn is_chapter_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 85 {
        return false;
    }
    if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
        return true;
    }
    let re = Regex::new(
        r"(?i)^(?:cap[íi]tulo|chapter|parte|part|pr[óo]logo|prologue|ep[íi]logo|epilogue|introdu[çc][ãa]o|pref[áa]cio)\b(?:\s+(?:\d+|[ivxlcdm]+|um|dois|tr[êe]s|quatro|cinco|one|two|three))?(?:\s*[:\-–—.]\s*.*)?$",
    )
    .unwrap();
    re.is_match(trimmed)
}

fn normalize_header_key(line: &str) -> String {
    let digits_re = Regex::new(r"\d+").unwrap();
    digits_re
        .replace_all(&line.trim().to_lowercase(), "")
        .trim()
        .to_string()
}

