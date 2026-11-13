//! Comandos invocados pela interface (via `invoke`). Toda entrada é validada aqui
//! ou nas camadas abaixo antes de chegar a processos externos ou ao disco.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_plugin_opener::OpenerExt;

use crate::downloads::job::Job;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::storage::settings::Settings;
use crate::tools::{self, ToolInfo};
use crate::updater::{self, UpdateResult};
use crate::url_guard::{self, UrlCheck};
use crate::ytdlp::info::MediaInfo;
use crate::ytdlp::spec::{DownloadChoice, Preference};

type Res<T> = Result<T, AppError>;

// ---------------------------------------------------------------- configurações

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Res<Settings> {
    let s = settings.normalized(&state.default_download_dir);
    s.validate()?;
    state.settings_store.save(&s)?;
    *state.settings.write().unwrap() = s.clone();
    // Um limite maior de simultâneos pode liberar novas vagas imediatamente.
    state.manager.pump();
    Ok(s)
}

// ------------------------------------------------------------------- links

#[tauri::command]
pub fn check_url(text: String) -> UrlCheck {
    url_guard::check(&text)
}

#[derive(Serialize)]
pub struct ExtractedUrls {
    valid: Vec<String>,
    invalid: Vec<String>,
}

#[tauri::command]
pub fn extract_urls(text: String) -> ExtractedUrls {
    let (valid, invalid) = url_guard::extract_many(&text);
    ExtractedUrls { valid, invalid }
}

#[tauri::command]
pub async fn analyze_url(state: State<'_, AppState>, url: String) -> Res<MediaInfo> {
    let manager = state.manager.clone();
    manager.analyze(&url).await
}

// ------------------------------------------------------------------- fila

#[tauri::command]
pub fn enqueue_download(
    state: State<'_, AppState>,
    url: String,
    choice: DownloadChoice,
    dest_dir: Option<String>,
) -> Res<Job> {
    state.manager.enqueue(&url, Some(choice), None, dest_dir)
}

#[tauri::command]
pub fn enqueue_many(
    state: State<'_, AppState>,
    urls: Vec<String>,
    preference: Option<Preference>,
    dest_dir: Option<String>,
) -> Res<Vec<Job>> {
    if urls.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidUrl, "Nenhum link válido foi informado."));
    }
    state.manager.enqueue_many(&urls, preference, dest_dir)
}

#[tauri::command]
pub fn list_queue(state: State<'_, AppState>) -> Vec<Job> {
    state.manager.queue()
}

#[tauri::command]
pub fn pause_job(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.pause(&id)
}

#[tauri::command]
pub fn resume_job(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.resume(&id)
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.cancel(&id)
}

#[tauri::command]
pub fn retry_job(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.retry(&id)
}

#[tauri::command]
pub fn remove_job(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.remove(&id)
}

#[tauri::command]
pub fn clear_finished(state: State<'_, AppState>) -> Res<usize> {
    state.manager.clear_finished()
}

#[tauri::command]
pub fn active_count(state: State<'_, AppState>) -> usize {
    state.manager.active_count()
}

// --------------------------------------------------------------- histórico

#[tauri::command]
pub fn list_history(state: State<'_, AppState>, search: Option<String>) -> Res<Vec<Job>> {
    state.manager.history(search.as_deref())
}

#[tauri::command]
pub fn remove_history(state: State<'_, AppState>, id: String) -> Res<()> {
    state.manager.remove_history(&id)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Res<()> {
    state.manager.clear_history()
}

#[tauri::command]
pub fn redownload(state: State<'_, AppState>, id: String) -> Res<Job> {
    state.manager.redownload(&id)
}

// ------------------------------------------------------- arquivos e pastas

fn open_err(e: impl std::fmt::Display) -> AppError {
    AppError::internal("O Windows não conseguiu abrir este item.").with_detail(e.to_string())
}

#[tauri::command]
pub fn open_job_file(app: AppHandle, state: State<'_, AppState>, id: String) -> Res<()> {
    let p = state.manager.file_of(&id)?;
    app.opener().open_path(p.display().to_string(), None::<&str>).map_err(open_err)
}

#[tauri::command]
pub fn reveal_job_file(app: AppHandle, state: State<'_, AppState>, id: String) -> Res<()> {
    match state.manager.file_of(&id) {
        Ok(p) => app.opener().reveal_item_in_dir(p).map_err(open_err),
        Err(_) => {
            let dir = state.manager.folder_of(&id)?;
            app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(open_err)
        }
    }
}

/// Abre uma pasta (usado para a pasta padrão de downloads). Apenas diretórios existentes.
#[tauri::command]
pub fn open_folder(app: AppHandle, path: String) -> Res<()> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() || !p.is_dir() {
        return Err(AppError::not_found("A pasta não existe."));
    }
    app.opener().open_path(p.display().to_string(), None::<&str>).map_err(open_err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    path: String,
    exists: bool,
    free_bytes: Option<u64>,
}

#[tauri::command]
pub fn folder_info(path: String) -> FolderInfo {
    let p = PathBuf::from(&path);
    FolderInfo { exists: p.is_dir(), free_bytes: crate::system::free_space(&p), path }
}

/// Abre páginas fixas (licenças/código-fonte dos componentes). Nunca URLs arbitrárias.
#[tauri::command]
pub fn open_known_link(app: AppHandle, key: String) -> Res<()> {
    let url = match key.as_str() {
        "ytdlp" => "https://github.com/yt-dlp/yt-dlp",
        "ffmpeg" => "https://ffmpeg.org/legal.html",
        "ffmpeg-source" => "https://github.com/yt-dlp/FFmpeg-Builds",
        "deno" => "https://github.com/denoland/deno",
        "tauri" => "https://tauri.app",
        _ => return Err(AppError::not_found("Link desconhecido.")),
    };
    app.opener().open_url(url, None::<&str>).map_err(open_err)
}

// ------------------------------------------------------------- componentes

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    tools: Option<ToolInfo>,
    tools_error: Option<AppError>,
    data_dir: String,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> Res<AppInfo> {
    let tools = state.manager.tools();
    let info = tools.info().await;
    let ok = info.ytdlp_version.is_some();
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        tools: Some(info),
        tools_error: if ok {
            None
        } else {
            Some(AppError::new(ErrorCode::ToolMissing, "O mecanismo de download não respondeu. Reinstale o Vibax."))
        },
        data_dir: state.local_data.display().to_string(),
    })
}

fn reload_tools(state: &AppState) -> AppResult<()> {
    let paths = tools::resolve(state.resource_dir.as_deref(), &state.local_data, state.cache_dir.clone())?;
    state.manager.set_tools(Arc::new(paths));
    Ok(())
}

#[tauri::command]
pub async fn update_ytdlp(state: State<'_, AppState>) -> Res<UpdateResult> {
    if state.manager.active_count() > 0 {
        return Err(AppError::new(ErrorCode::Busy, "Aguarde os downloads em andamento terminarem para atualizar."));
    }
    let current = state.manager.tools().info().await.ytdlp_version;
    let r = updater::update(&state.local_data, current).await?;
    if r.updated {
        reload_tools(&state)?;
    }
    Ok(r)
}

#[tauri::command]
pub fn reset_ytdlp(state: State<'_, AppState>) -> Res<()> {
    if state.manager.active_count() > 0 {
        return Err(AppError::new(ErrorCode::Busy, "Aguarde os downloads em andamento terminarem."));
    }
    updater::reset(&state.local_data)?;
    reload_tools(&state)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.manager.shutdown();
    }
    app.exit(0);
}
