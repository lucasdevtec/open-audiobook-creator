use thiserror::Error;

#[derive(Error, Debug)]
pub enum OacError {
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato de arquivo não suportado: {0}")]
    UnsupportedFormat(String),

    #[error("Falha ao processar PDF: {0}")]
    PdfError(String),

    #[error("Falha ao processar EPUB: {0}")]
    EpubError(String),

    #[error("Falha na serialização do projeto: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Erro no motor TTS ({engine}): {message}")]
    TtsError { engine: String, message: String },

    #[error("Erro na masterização de áudio: {0}")]
    AudioError(String),

    #[error("Capítulo não encontrado: {0}")]
    ChapterNotFound(String),
}

pub type Result<T> = std::result::Result<T, OacError>;

