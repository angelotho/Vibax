use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter, Manager as _};
use tauri_plugin_notification::NotificationExt;

use crate::downloads::job::{Job, JobStatus};
use crate::downloads::manager::{EventSink, Manager};
use crate::storage::settings::{NotifyMode, Settings, SettingsStore};

pub struct AppState {
    pub manager: Arc<Manager>,
    pub settings: Arc<RwLock<Settings>>,
    pub settings_store: SettingsStore,
    pub default_download_dir: PathBuf,
    pub local_data: PathBuf,
    pub resource_dir: Option<PathBuf>,
    pub cache_dir: PathBuf,
}

pub const EVT_JOB_UPDATED: &str = "job-updated";
pub const EVT_JOB_REMOVED: &str = "job-removed";
pub const EVT_CLOSE_REQUESTED: &str = "close-requested";

/// Encaminha eventos do gerenciador para a janela e para as notificações do Windows.
pub struct AppSink {
    pub app: AppHandle,
    pub settings: Arc<RwLock<Settings>>,
}

impl EventSink for AppSink {
    fn job_updated(&self, job: &Job) {
        let _ = self.app.emit(EVT_JOB_UPDATED, job);
    }

    fn job_removed(&self, id: &str) {
        let _ = self.app.emit(EVT_JOB_REMOVED, id);
    }

    fn job_finished(&self, job: &Job) {
        let mode = self.settings.read().unwrap().notifications;
        let focused = self
            .app
            .get_webview_window("main")
            .and_then(|w| w.is_focused().ok())
            .unwrap_or(false);
        let show = match mode {
            NotifyMode::Never => false,
            NotifyMode::Always => true,
            NotifyMode::Background => !focused,
        };
        if !show {
            return;
        }
        let title_text = job.title.clone().unwrap_or_else(|| "Download".into());
        let (title, body) = match job.status {
            JobStatus::Completed => ("Download concluído".to_string(), title_text),
            JobStatus::Error => (
                "Falha no download".to_string(),
                format!("{title_text}\n{}", job.error.as_ref().map(|e| e.message.as_str()).unwrap_or("")),
            ),
            _ => return,
        };
        let _ = self.app.notification().builder().title(title).body(body).show();
    }
}
