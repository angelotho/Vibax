use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::ytdlp::spec::{DownloadChoice, Preference};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Aguardando vaga na fila.
    Queued,
    /// Obtendo informações do link (itens adicionados em lote).
    Analyzing,
    Downloading,
    /// Pós-processamento: mesclagem, conversão, metadados.
    Processing,
    Paused,
    Completed,
    Cancelled,
    Error,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Analyzing => "analyzing",
            JobStatus::Downloading => "downloading",
            JobStatus::Processing => "processing",
            JobStatus::Paused => "paused",
            JobStatus::Completed => "completed",
            JobStatus::Cancelled => "cancelled",
            JobStatus::Error => "error",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, JobStatus::Analyzing | JobStatus::Downloading | JobStatus::Processing)
    }

    pub fn is_finished(self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Cancelled | JobStatus::Error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    /// Miniatura salva localmente (exibida mesmo offline no histórico).
    pub thumb_path: Option<String>,
    pub source_label: Option<String>,
    pub duration: Option<f64>,

    pub choice: Option<DownloadChoice>,
    pub preference: Option<Preference>,
    /// "MP4", "MP3"… (atualizado com a extensão real ao concluir).
    pub format_label: String,
    pub quality_label: Option<String>,
    pub dest_dir: String,

    pub status: JobStatus,
    pub stage: Option<String>,
    pub progress: Option<f64>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,

    pub file_path: Option<String>,
    pub file_size: Option<u64>,
    pub error: Option<AppError>,

    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub in_queue: bool,
    pub in_history: bool,

    /// Calculado ao listar o histórico.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_exists: Option<bool>,
}

impl Job {
    pub fn reset_progress(&mut self) {
        self.progress = None;
        self.downloaded_bytes = 0;
        self.speed = None;
        self.eta = None;
        self.stage = None;
        self.error = None;
        self.finished_at = None;
    }

    pub fn needs_merge(&self) -> bool {
        matches!(&self.choice, Some(DownloadChoice::Video { video_has_audio: false, .. }))
    }
}
