use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Narrator,
    Dialogue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    pub text: String,
    pub kind: SegmentKind,
}

static RE_QUOTED_DIALOGUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)(["“«][^"”»]+["”»])"#).unwrap()
});

static RE_INLINE_SPEECH_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\s*[—–\-]\s*(disse|falou|perguntou|respondeu|gritou|murmurou|sussurrou|exclamou|retrucou|indagou|comentou|pensou|afirmou|acrescentou|observou|continuou|interrompeu)\b"#).unwrap()
});

/// Divide um texto em segmentos classificados como Narrador ou Diálogo.
pub fn segment_dialogues(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();

    // Processar parágrafo por parágrafo
    for raw_para in text.split("\n\n") {
        let para = raw_para.trim();
        if para.is_empty() {
            continue;
        }

        // Caso 1: Parágrafo iniciado por travessão de diálogo (— ou – ou -)
        if para.starts_with('—') || para.starts_with('–') || (para.starts_with('-') && para.chars().nth(1) == Some(' ')) {
            // Remove o travessão inicial
            let content = para.trim_start_matches(|c: char| c == '—' || c == '–' || c == '-' || c.is_whitespace());

            // Verificar se há quebra para fala de narrador com travessão intercalado:
            // Ex: "Não podemos recuar — disse ele com firmeza — antes do amanhecer."
            let parts: Vec<&str> = content.split(|c| c == '—' || c == '–').collect();

            if parts.len() > 1 {
                for (idx, part) in parts.iter().enumerate() {
                    let trimmed = part.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Se for a segunda parte e começar com verbo dicendi (disse ele...)
                    let is_speech_tag = idx == 1 && RE_INLINE_SPEECH_TAG.is_match(&format!("— {trimmed}"));

                    let kind = if is_speech_tag {
                        SegmentKind::Narrator
                    } else if idx % 2 == 0 {
                        SegmentKind::Dialogue
                    } else {
                        SegmentKind::Narrator
                    };

                    segments.push(TextSegment {
                        text: trimmed.to_string(),
                        kind,
                    });
                }
            } else {
                segments.push(TextSegment {
                    text: content.to_string(),
                    kind: SegmentKind::Dialogue,
                });
            }
            continue;
        }

        // Caso 2: Falas entre aspas ("...", “...”, «...») dentro do parágrafo
        if para.contains('"') || para.contains('“') || para.contains('«') {
            let mut last_idx = 0;
            for mat in RE_QUOTED_DIALOGUE.find_iter(para) {
                let start = mat.start();
                let end = mat.end();

                if start > last_idx {
                    let narrative_text = para[last_idx..start].trim();
                    if !narrative_text.is_empty() {
                        segments.push(TextSegment {
                            text: narrative_text.to_string(),
                            kind: SegmentKind::Narrator,
                        });
                    }
                }

                let quote_trimmed = mat.as_str().trim_matches(|c: char| {
                    c == '"' || c == '“' || c == '”' || c == '«' || c == '»' || c.is_whitespace()
                });

                if !quote_trimmed.is_empty() {
                    segments.push(TextSegment {
                        text: quote_trimmed.to_string(),
                        kind: SegmentKind::Dialogue,
                    });
                }

                last_idx = end;
            }

            if last_idx < para.len() {
                let remainder = para[last_idx..].trim();
                if !remainder.is_empty() {
                    segments.push(TextSegment {
                        text: remainder.to_string(),
                        kind: SegmentKind::Narrator,
                    });
                }
            }
            continue;
        }

        // Caso padrão: Todo o parágrafo é narração
        segments.push(TextSegment {
            text: para.to_string(),
            kind: SegmentKind::Narrator,
        });
    }

    if segments.is_empty() && !text.trim().is_empty() {
        segments.push(TextSegment {
            text: text.trim().to_string(),
            kind: SegmentKind::Narrator,
        });
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quoted_dialogue() {
        let text = "O vento soprava forte. \"Você ouviu isso?\", perguntou Lucas com os olhos arregalados.";
        let segs = segment_dialogues(text);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].kind, SegmentKind::Narrator);
        assert_eq!(segs[1].kind, SegmentKind::Dialogue);
        assert_eq!(segs[1].text, "Você ouviu isso?");
        assert_eq!(segs[2].kind, SegmentKind::Narrator);
    }

    #[test]
    fn test_dash_dialogue() {
        let text = "— Precisamos sair daqui agora — disse ela.";
        let segs = segment_dialogues(text);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].kind, SegmentKind::Dialogue);
        assert_eq!(segs[0].text, "Precisamos sair daqui agora");
        assert_eq!(segs[1].kind, SegmentKind::Narrator);
        assert_eq!(segs[1].text, "disse ela.");
    }
}
