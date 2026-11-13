//! Erros da aplicação, serializados para a interface com um código estável
//! (usado pela UI para escolher ícones/ações) e uma mensagem em português.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidUrl,
    Unsupported,
    Network,
    Unavailable,
    AuthRequired,
    Drm,
    GeoBlocked,
    Live,
    Forbidden,
    NoFormats,
    DiskFull,
    Permission,
    Processing,
    ToolMissing,
    NotFound,
    Busy,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    /// Trecho técnico (última linha de erro da ferramenta), exibido sob demanda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), detail: None }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        let d = detail.into();
        if !d.trim().is_empty() {
            self.detail = Some(d);
        }
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind::*;
        match e.kind() {
            PermissionDenied => AppError::new(
                ErrorCode::Permission,
                "Sem permissão para acessar o arquivo ou a pasta.",
            )
            .with_detail(e.to_string()),
            NotFound => AppError::not_found("Arquivo ou pasta não encontrado.").with_detail(e.to_string()),
            StorageFull => AppError::new(ErrorCode::DiskFull, "Não há espaço suficiente no disco.")
                .with_detail(e.to_string()),
            _ => AppError::internal("Falha de entrada/saída.").with_detail(e.to_string()),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::internal("Falha ao acessar o armazenamento local.").with_detail(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::internal("Dados locais em formato inválido.").with_detail(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
