//! Vibax — backend Rust.
//!
//! Camadas:
//! - `commands`   : fronteira com a interface (validação de entrada);
//! - `downloads`  : fila, agendamento e ciclo de vida das tarefas;
//! - `ytdlp`      : análise de links, montagem de argumentos e leitura de progresso;
//! - `storage`    : SQLite (fila/histórico) e configurações em JSON;
//! - `tools`/`updater` : localização e atualização dos componentes externos;
//! - `process`/`system`: recursos nativos do Windows.

pub mod commands;
pub mod downloads;
pub mod error;
pub mod process;
pub mod state;
pub mod storage;
pub mod system;
pub mod tools;
pub mod updater;
pub mod url_guard;
pub mod ytdlp;

use std::sync::{Arc, RwLock};

use tauri::{Emitter, Manager as _, RunEvent, WindowEvent};

use downloads::manager::Manager;
use state::{AppSink, AppState, EVT_CLOSE_REQUESTED};
use storage::db::Db;
use storage::settings::SettingsStore;

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let paths = app.path();
    let local_data = paths.app_local_data_dir()?;
    let config_dir = paths.app_config_dir()?;
    let cache_dir = paths.app_cache_dir()?;
    let resource_dir = paths.resource_dir().ok();
    let default_download_dir = paths
        .download_dir()
        .or_else(|_| paths.home_dir().map(|h| h.join("Downloads")))?
        .join("Vibax");
    std::fs::create_dir_all(&local_data)?;
    std::fs::create_dir_all(&cache_dir)?;

    let settings_store = SettingsStore::new(config_dir.join("settings.json"), default_download_dir.clone());
    let settings = Arc::new(RwLock::new(settings_store.load()));

    let ytdlp_cache = cache_dir.join("yt-dlp");
    let tools = match tools::resolve(resource_dir.as_deref(), &local_data, ytdlp_cache.clone()) {
        Ok(t) => t,
        Err(e) => {
            // O app abre mesmo assim e mostra o problema na interface.
            log::error!("componentes ausentes: {}", e.message);
            let base = resource_dir.clone().unwrap_or_default().join("tools");
            tools::ToolPaths {
                ytdlp: base.join("yt-dlp").join(tools::YTDLP_EXE),
                ffmpeg_dir: base.clone(),
                deno: base.join(tools::DENO_EXE),
                ytdlp_updated: false,
                cache_dir: ytdlp_cache,
            }
        }
    };

    let db = Arc::new(Db::open(&local_data.join("vibax.db"))?);
    let sink = Arc::new(AppSink { app: app.handle().clone(), settings: settings.clone() });
    let manager = Manager::new(db, sink, settings.clone(), Arc::new(tools), Some(local_data.join("thumbs")));
    manager.restore()?;

    app.manage(AppState {
        manager,
        settings,
        settings_store,
        default_download_dir,
        local_data,
        resource_dir,
        cache_dir,
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(setup)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Com downloads em andamento, a interface pede confirmação antes de fechar.
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if state.manager.active_count() > 0 {
                        api.prevent_close();
                        let _ = window.emit(EVT_CLOSE_REQUESTED, state.manager.active_count());
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::check_url,
            commands::extract_urls,
            commands::analyze_url,
            commands::enqueue_download,
            commands::enqueue_many,
            commands::list_queue,
            commands::pause_job,
            commands::resume_job,
            commands::cancel_job,
            commands::retry_job,
            commands::remove_job,
            commands::clear_finished,
            commands::active_count,
            commands::list_history,
            commands::remove_history,
            commands::clear_history,
            commands::redownload,
            commands::open_job_file,
            commands::reveal_job_file,
            commands::open_folder,
            commands::folder_info,
            commands::open_known_link,
            commands::app_info,
            commands::update_ytdlp,
            commands::reset_ytdlp,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("falha ao iniciar o Vibax");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.manager.shutdown();
            }
        }
    });
}
