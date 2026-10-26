use crate::error::{OacError, Result};
use crate::model::{BookMetadata, SourceFormat};
use crate::parser::{ExtractedDocument, RawPage};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};

pub fn extract_pdf_bytes(bytes: &[u8]) -> Result<ExtractedDocument> {
    let doc = Document::load_mem(bytes).map_err(|e| OacError::PdfError(e.to_string()))?;
    let mut metadata = BookMetadata::default();

    // Extract PDF Info metadata if available
    if let Ok(Object::Reference(info_ref)) = doc.trailer.get(b"Info") {
        if let Ok(Object::Dictionary(info_dict)) = doc.get_object(*info_ref) {
            if let Ok(title_obj) = info_dict.get(b"Title") {
                if let Some(s) = pdf_object_to_string(title_obj) {
                    if !s.trim().is_empty() {
                        metadata.title = s.trim().to_string();
                    }
                }
            }
            if let Ok(author_obj) = info_dict.get(b"Author") {
                if let Some(s) = pdf_object_to_string(author_obj) {
                    if !s.trim().is_empty() {
                        metadata.author = s.trim().to_string();
                    }
                }
            }
        }
    }

    let pages_map = doc.get_pages();
    let mut pages = Vec::with_capacity(pages_map.len());

    for (page_num, page_id) in pages_map {
        let mut page_text = doc.extract_text(&[page_num]).unwrap_or_default();

        // Fallback: if lopdf::extract_text returned empty or garbled, inspect content stream operations directly
        if page_text.trim().is_empty() {
            if let Ok(content_bytes) = doc.get_page_content(page_id) {
                if let Ok(content) = Content::decode(&content_bytes) {
                    page_text = decode_operations_text(&content.operations);
                }
            }
        }

        pages.push(RawPage {
            page_number: page_num,
            chapter_hint: None,
            text: page_text,
        });
    }

    if pages.is_empty() {
        return Err(OacError::PdfError(
            "O arquivo PDF não contém páginas legíveis.".to_string(),
        ));
    }

    Ok(ExtractedDocument {
        format: SourceFormat::Pdf,
        metadata,
        pages,
    })
}

fn pdf_object_to_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(decode_pdf_bytes(bytes)),
        _ => None,
    }
}

fn decode_pdf_bytes(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else if let Ok(utf8) = std::str::from_utf8(bytes) {
        utf8.to_string()
    } else {
        // ISO-8859-1 / WinAnsi fallback common in Portuguese PDFs
        bytes.iter().map(|&b| b as char).collect()
    }
}

fn decode_operations_text(operations: &[Operation]) -> String {
    let mut out = String::new();
    for op in operations {
        match op.operator.as_str() {
            "Tj" | "'" | "\"" => {
                for operand in &op.operands {
                    if let Object::String(bytes, _) = operand {
                        out.push_str(&decode_pdf_bytes(bytes));
                    }
                }
                out.push('\n');
            }
            "TJ" => {
                for operand in &op.operands {
                    if let Object::Array(items) = operand {
                        for item in items {
                            if let Object::String(bytes, _) = item {
                                out.push_str(&decode_pdf_bytes(bytes));
                            }
                        }
                    }
                }
                out.push('\n');
            }
            "Td" | "TD" | "T*" => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
    out
}

/// Gera um arquivo PDF válido em memória com múltiplas páginas, cabeçalhos repetidos,
/// numeração de páginas, palavras hifenizadas e capítulos para demonstração e testes.
pub fn generate_sample_pdf_bytes() -> Result<Vec<u8>> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let info_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal("A Arquitetura do Silêncio Digital"),
        "Author" => Object::string_literal("Dra. Helena Vasconcelos"),
    });
    doc.trailer.set("Info", info_id);

    let sample_pages: Vec<Vec<&str>> = vec![
        vec![
            "A ARQUITETURA DO SILENCIO DIGITAL - HELENA VASCONCELOS",
            "Capítulo 1: O Despertar das Máquinas Leitoras",
            "Quando os primeiros livros foram digitalizados no final do século vinte,",
            "poucos imaginavam que a voz sintética alcançaria tamanha naturali-",
            "dade emocional e clareza acústica.",
            "",
            "A transformação de documentos estáticos em experiências sonoras",
            "permite que o conhecimento acompanhe o leitor em qualquer jornada.",
            "Página 1",
        ],
        vec![
            "A ARQUITETURA DO SILENCIO DIGITAL - HELENA VASCONCELOS",
            "Contudo, o formato PDF foi concebido como um papel virtual fixo,",
            "repleto de cabeçalhos repetitivos, notas marginais e palavras que-",
            "bradas no final de cada linha impressa.",
            "",
            "Por isso, um sistema inteligente precisa higienizar o texto antes",
            "de entregá-lo ao motor neural de síntese de voz.",
            "Página 2",
        ],
        vec![
            "A ARQUITETURA DO SILENCIO DIGITAL - HELENA VASCONCELOS",
            "Capítulo 2: Vozes Locais e Soberania Aberta",
            "Executar modelos como Piper e Kokoro diretamente no computador do",
            "usuário garante privacidade total e independência de servidores",
            "externos pagos.",
            "",
            "Além disso, o empacotamento em formato M4B preserva a divisão",
            "exata de capítulos, permitindo navegar pela obra com facilidade.",
            "Página 3",
        ],
        vec![
            "A ARQUITETURA DO SILENCIO DIGITAL - HELENA VASCONCELOS",
            "Capítulo 3: O Futuro dos Audiolivros Livres",
            "Com ferramentas de código aberto multiplataforma em Rust, criadores,",
            "estudantes e pessoas com deficiência visual ganham autonomia para",
            "converter bibliotecas inteiras em áudio de alta definição.",
            "",
            "O conhecimento humano, enfim, ganha voz própria e acessível.",
            "Página 4",
        ],
    ];

    let mut page_ids = Vec::new();

    for lines in sample_pages {
        let mut ops = vec![
            Operation::new("BT", vec![]),
            Operation::new("F1", vec![12.into()]),
            Operation::new("Td", vec![50.into(), 750.into()]),
        ];

        for line in lines {
            // Encode UTF-8 string into ISO-8859-1 / WinAnsi bytes for PDF Type1 font compatibility
            let ansi_bytes: Vec<u8> = line
                .chars()
                .map(|ch| if (ch as u32) <= 0xFF { ch as u8 } else { b'?' })
                .collect();
            ops.push(Operation::new(
                "Tj",
                vec![Object::String(ansi_bytes, lopdf::StringFormat::Literal)],
            ));
            ops.push(Operation::new("Td", vec![0.into(), (-18).into()]));
        }
        ops.push(Operation::new("ET", vec![]));

        let content = Content { operations: ops };
        let content_stream = Stream::new(dictionary! {}, content.encode().unwrap_or_default());
        let content_id = doc.add_object(content_stream);

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        page_ids.push(page_id.into());
    }

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids,
        "Count" => 4,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)
        .map_err(|e| OacError::PdfError(e.to_string()))?;
    Ok(buffer)
}

