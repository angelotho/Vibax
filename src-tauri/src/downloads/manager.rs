//! Gerenciador da fila de downloads.
//!
//! - Agenda tarefas respeitando o limite de downloads simultâneos.
//! - Cada tarefa roda em uma task assíncrona; a interface nunca é bloqueada.
//! - Estado em memória espelhado no SQLite a cada mudança relevante, de forma
//!   que a fila sobrevive ao fechamento (ou queda) do aplicativo.
//! - Pausa = encerra o processo mantendo os arquivos parciais (.part); ao retomar
//!   o yt-dlp continua do ponto em que parou. Cancelar remove os parciais.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tokio::sync::watch;

use super::job::{Job, JobStatus};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::storage::db::Db;
use crate::storage::settings::{DefaultKind, Settings};
use crate::system;
use crate::tools::ToolPaths;
use crate::ytdlp::info::{AudioFormat, MediaInfo, MediaKind};
use crate::ytdlp::spec::{self, ArgsContext, DownloadChoice, Preference};
use crate::ytdlp::{Engine, RunError, StopReason, Update};

/// Destino dos eventos (a janela do app em produção; um coletor nos testes).
pub trait EventSink: Send + Sync + 'static {
    fn job_updated(&self, job: &Job);
    fn job_removed(&self, id: &str);
    /// Chamado ao concluir ou falhar (notificações do sistema).
    fn job_finished(&self, _job: &Job) {}
}

enum Outcome {
    Completed(PathBuf),
    Stopped(StopReason),
    Failed(AppError),
}

#[derive(Default)]
struct State {
    jobs: Vec<Job>,
    running: HashMap<String, watch::Sender<Option<StopReason>>>,
    last_emit: HashMap<String, Instant>,
}

pub struct Manager {
    db: Arc<Db>,
    sink: Arc<dyn EventSink>,
    settings: Arc<RwLock<Settings>>,
    tools: RwLock<Arc<ToolPaths>>,
    thumbs_dir: Option<PathBuf>,
    http: reqwest::Client,
    state: Mutex<State>,
    probe_cache: Mutex<Vec<(String, MediaInfo)>>,
    shutting_down: AtomicBool,
}

const EMIT_INTERVAL: Duration = Duration::from_millis(200);
const PROBE_CACHE_SIZE: usize = 24;
const MAX_ATTEMPTS: u32 = 3;
const INTERRUPTED: &str = "Interrompido quando o Vibax foi fechado";

impl Manager {
    pub fn new(
        db: Arc<Db>,
        sink: Arc<dyn EventSink>,
        settings: Arc<RwLock<Settings>>,
        tools: Arc<ToolPaths>,
        thumbs_dir: Option<PathBuf>,
    ) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Vibax/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        Arc::new(Self {
            db,
            sink,
            settings,
            tools: RwLock::new(tools),
            thumbs_dir,
            http,
            state: Mutex::new(State::default()),
            probe_cache: Mutex::new(Vec::new()),
            shutting_down: AtomicBool::new(false),
        })
    }

    fn st(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn tools(&self) -> Arc<ToolPaths> {
        self.tools.read().unwrap().clone()
    }

    pub fn set_tools(&self, tools: Arc<ToolPaths>) {
        *self.tools.write().unwrap() = tools;
    }

    fn engine(&self) -> Engine {
        Engine::new(self.tools())
    }

    fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    /// Carrega a fila salva. Tarefas que estavam em andamento quando o app
    /// fechou viram "pausadas" para que o usuário possa retomá-las.
    pub fn restore(self: &Arc<Self>) -> AppResult<()> {
        let mut jobs = self.db.queue()?;
        for j in jobs.iter_mut() {
            if j.status.is_active() {
                j.status = JobStatus::Paused;
                j.stage = Some(INTERRUPTED.into());
                j.speed = None;
                j.eta = None;
                self.db.upsert(j)?;
            }
        }
        self.st().jobs = jobs;
        self.pump();
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.st().running.len()
    }

    pub fn queue(&self) -> Vec<Job> {
        self.st().jobs.clone()
    }

    pub fn get(&self, id: &str) -> Option<Job> {
        self.st().jobs.iter().find(|j| j.id == id).cloned().or_else(|| self.db.get(id).ok().flatten())
    }

    // ---------------------------------------------------------------- análise

    pub async fn analyze(&self, url: &str) -> AppResult<MediaInfo> {
        let url = crate::url_guard::validate(url)?;
        let info = self.engine().probe(&url).await?;
        let mut cache = self.probe_cache.lock().unwrap();
        cache.retain(|(u, _)| u != &url);
        cache.push((url, info.clone()));
        if cache.len() > PROBE_CACHE_SIZE {
            cache.remove(0);
        }
        Ok(info)
    }

    fn cached_info(&self, url: &str) -> Option<MediaInfo> {
        self.probe_cache
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(u, i)| u == url || i.url == url)
            .map(|(_, i)| i.clone())
    }

    // ------------------------------------------------------------ enfileirar

    pub fn default_preference(&self) -> Preference {
        let s = self.settings();
        match s.default_kind {
            DefaultKind::Video => Preference::Video { max_height: s.max_height(), container: s.default_container },
            DefaultKind::Audio => Preference::Audio {
                format: s.default_audio_format,
                bitrate: (s.default_audio_format == AudioFormat::Mp3).then_some(s.mp3_bitrate),
            },
        }
    }

    fn resolve_dest(&self, dest_dir: Option<String>) -> AppResult<String> {
        let dir = dest_dir.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| self.settings().download_dir);
        if !Path::new(&dir).is_absolute() {
            return Err(AppError::new(ErrorCode::Permission, "Escolha uma pasta de destino válida."));
        }
        Ok(dir)
    }

    /// Adiciona um item à fila. Com `choice` (vinda da prévia) o download começa
    /// direto; sem ela, o item passa por "analisando" e usa `preference`.
    pub fn enqueue(
        self: &Arc<Self>,
        url: &str,
        choice: Option<DownloadChoice>,
        preference: Option<Preference>,
        dest_dir: Option<String>,
    ) -> AppResult<Job> {
        let url = crate::url_guard::validate(url)?;
        if let Some(c) = &choice {
            c.validate()?;
        }
        let dest_dir = self.resolve_dest(dest_dir)?;
        let info = self.cached_info(&url).filter(|i| i.kind == MediaKind::Video);
        let preference = preference.or_else(|| choice.as_ref().map(preference_from_choice)).unwrap_or_else(|| self.default_preference());

        let (format_label, quality_label) = match &choice {
            Some(c) => (c.ext_hint().to_uppercase(), Some(c.quality_label())),
            None => match &preference {
                Preference::Video { container, .. } => (container.ext().to_uppercase(), None),
                Preference::Audio { format, .. } => (format.ext().to_uppercase(), None),
            },
        };

        let job = Job {
            id: uuid::Uuid::new_v4().simple().to_string(),
            url: info.as_ref().map(|i| i.url.clone()).filter(|u| crate::url_guard::validate(u).is_ok()).unwrap_or(url),
            title: info.as_ref().map(|i| i.title.clone()),
            uploader: info.as_ref().and_then(|i| i.uploader.clone()),
            thumbnail: info.as_ref().and_then(|i| i.thumbnail.clone()),
            thumb_path: None,
            source_label: info.as_ref().map(|i| i.source_label.clone()),
            duration: info.as_ref().and_then(|i| i.duration),
            total_bytes: match (&info, &choice) {
                (Some(i), Some(c)) => estimate_size(i, c),
                _ => None,
            },
            choice: if info.is_some() { choice } else { None },
            preference: Some(preference),
            format_label,
            quality_label,
            dest_dir,
            status: JobStatus::Queued,
            stage: None,
            progress: None,
            downloaded_bytes: 0,
            speed: None,
            eta: None,
            file_path: None,
            file_size: None,
            error: None,
            created_at: system::now_ms(),
            started_at: None,
            finished_at: None,
            in_queue: true,
            in_history: false,
            file_exists: None,
        };
        self.db.upsert(&job)?;
        self.st().jobs.push(job.clone());
        self.sink.job_updated(&job);
        if let Some(t) = job.thumbnail.clone() {
            self.fetch_thumbnail(job.id.clone(), t);
        }
        self.pump();
        Ok(job)
    }

    /// Vários links de uma vez (texto colado ou itens de playlist).
    pub fn enqueue_many(
        self: &Arc<Self>,
        urls: &[String],
        preference: Option<Preference>,
        dest_dir: Option<String>,
    ) -> AppResult<Vec<Job>> {
        let pref = preference.unwrap_or_else(|| self.default_preference());
        let mut out = Vec::new();
        for u in urls.iter().take(500) {
            out.push(self.enqueue(u, None, Some(pref.clone()), dest_dir.clone())?);
        }
        Ok(out)
    }

    // ------------------------------------------------------------ agendador

    /// Inicia tarefas aguardando enquanto houver vagas.
    pub fn pump(self: &Arc<Self>) {
        let max = self.settings().max_concurrent.max(1) as usize;
        let mut starts = Vec::new();
        {
            let mut st = self.st();
            let mut free = max.saturating_sub(st.running.len());
            let ids: Vec<String> = st
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Queued)
                .map(|j| j.id.clone())
                .collect();
            for id in ids {
                if free == 0 {
                    break;
                }
                if st.running.contains_key(&id) {
                    continue;
                }
                let (tx, rx) = watch::channel(None);
                st.running.insert(id.clone(), tx);
                starts.push((id, rx));
                free -= 1;
            }
        }
        for (id, rx) in starts {
            let me = self.clone();
            tauri::async_runtime::spawn(async move { me.run_job(id, rx).await });
        }
    }

    fn update(&self, id: &str, emit: bool, f: impl FnOnce(&mut Job)) -> Option<Job> {
        let mut st = self.st();
        let job = st.jobs.iter_mut().find(|j| j.id == id)?;
        f(job);
        let snapshot = job.clone();
        if emit {
            st.last_emit.insert(id.to_string(), Instant::now());
        }
        drop(st);
        if emit {
            self.sink.job_updated(&snapshot);
        }
        Some(snapshot)
    }

    fn persist(&self, job: &Job) {
        if let Err(e) = self.db.upsert(job) {
            log::error!("falha ao salvar tarefa {}: {e}", job.id);
        }
    }

    async fn run_job(self: Arc<Self>, id: String, rx: watch::Receiver<Option<StopReason>>) {
        let outcome = self.execute(&id, rx).await;
        self.finalize(&id, outcome);
        self.pump();
    }

    async fn execute(self: &Arc<Self>, id: &str, mut rx: watch::Receiver<Option<StopReason>>) -> Outcome {
        let Some(mut job) = self.update(id, true, |j| {
            j.started_at = Some(system::now_ms());
            j.error = None;
            j.status = if j.choice.is_none() { JobStatus::Analyzing } else { JobStatus::Downloading };
            j.stage = j.choice.is_none().then(|| "Analisando o link".to_string());
        }) else {
            return Outcome::Stopped(StopReason::Cancel);
        };
        self.persist(&job);

        // 1) Análise, para itens adicionados sem prévia.
        if job.choice.is_none() {
            let engine = self.engine();
            let probe = tokio::select! {
                r = engine.probe(&job.url) => r,
                r = wait_stop(&mut rx) => return Outcome::Stopped(r),
            };
            let info = match probe {
                Ok(i) => i,
                Err(e) => return Outcome::Failed(e),
            };
            if info.kind == MediaKind::Playlist {
                return Outcome::Failed(AppError::new(
                    ErrorCode::Unsupported,
                    "Este link é de uma playlist. Cole-o na tela principal para adicionar os vídeos dela.",
                ));
            }
            let pref = job.preference.clone().unwrap_or_else(|| self.default_preference());
            let choice = match pref.resolve(&info) {
                Ok(c) => c,
                Err(e) => return Outcome::Failed(e),
            };
            let thumb = info.thumbnail.clone();
            job = self
                .update(id, true, |j| {
                    j.title = Some(info.title.clone());
                    j.uploader = info.uploader.clone();
                    j.thumbnail = info.thumbnail.clone();
                    j.source_label = Some(info.source_label.clone());
                    j.duration = info.duration;
                    j.total_bytes = estimate_size(&info, &choice);
                    j.format_label = choice.ext_hint().to_uppercase();
                    j.quality_label = Some(choice.quality_label());
                    j.choice = Some(choice);
                    j.status = JobStatus::Downloading;
                    j.stage = None;
                })
                .unwrap_or(job);
            self.persist(&job);
            if let (Some(t), None) = (thumb, &job.thumb_path) {
                self.fetch_thumbnail(id.to_string(), t);
            }
        }

        // 2) Verificações de destino: pasta, permissão e espaço livre.
        let dest = PathBuf::from(&job.dest_dir);
        if let Err(e) = system::ensure_writable_dir(&dest) {
            return Outcome::Failed(e);
        }
        let remaining = job.total_bytes.map(|t| t.saturating_sub(job.downloaded_bytes));
        if let Err(e) = system::check_space(&dest, remaining, job.needs_merge()) {
            return Outcome::Failed(e);
        }
        let temp = match system::prepare_temp_dir(&dest, id) {
            Ok(t) => t,
            Err(e) => return Outcome::Failed(e),
        };

        // 3) Download. Falhas transitórias (rede, HTTP 403 intermitente da CDN)
        //    são repetidas automaticamente: o yt-dlp obtém novos endereços e
        //    continua dos arquivos parciais já baixados.
        let choice = job.choice.clone().expect("choice resolvida");
        let mut attempt = 1;
        loop {
            let tools = self.tools();
            let args = spec::build_download_args(
                tools.base_args(),
                &choice,
                &ArgsContext { url: &job.url, dest_dir: &dest, temp_dir: &temp, embed_metadata: self.settings().embed_metadata },
            );
            let engine = Engine::new(tools);
            let me = self.clone();
            let jid = id.to_string();
            let result = engine.download(args, rx.clone(), move |u| me.on_update(&jid, u)).await;
            match result {
                Ok(p) => return Outcome::Completed(p),
                Err(RunError::Stopped(r)) => return Outcome::Stopped(r),
                Err(RunError::Failed(e))
                    if attempt < MAX_ATTEMPTS && matches!(e.code, ErrorCode::Forbidden | ErrorCode::Network) =>
                {
                    attempt += 1;
                    log::info!("download {id}: nova tentativa {attempt}/{MAX_ATTEMPTS} após: {:?}", e.detail);
                    self.update(id, true, |j| {
                        j.status = JobStatus::Downloading;
                        j.stage = Some(format!("Reconectando (tentativa {attempt} de {MAX_ATTEMPTS})"));
                        j.speed = None;
                        j.eta = None;
                    });
                    let wait = Duration::from_secs(2 * attempt as u64);
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        r = wait_stop(&mut rx) => return Outcome::Stopped(r),
                    }
                }
                Err(RunError::Failed(e)) => return Outcome::Failed(e),
            }
        }
    }

    fn on_update(&self, id: &str, u: Update) {
        let throttled = {
            let st = self.st();
            st.last_emit.get(id).map(|t| t.elapsed() < EMIT_INTERVAL).unwrap_or(false)
        };
        match u {
            Update::Progress { fraction, downloaded, total, speed, eta } => {
                let finished_part = speed.is_none() && eta.is_none();
                self.update(id, !throttled || finished_part, |j| {
                    if j.status == JobStatus::Processing {
                        j.status = JobStatus::Downloading;
                        j.stage = None;
                    }
                    if fraction.is_some() {
                        j.progress = fraction;
                    }
                    j.downloaded_bytes = downloaded;
                    if total.is_some() {
                        j.total_bytes = total;
                    }
                    j.speed = speed;
                    j.eta = eta;
                });
            }
            Update::Stage(label) => {
                self.update(id, true, |j| {
                    j.status = JobStatus::Processing;
                    j.stage = Some(label.to_string());
                    j.speed = None;
                    j.eta = None;
                });
            }
            Update::Format { ext, size, parts } => {
                self.update(id, true, |j| {
                    if size.is_some() {
                        j.total_bytes = size;
                    }
                    j.stage = (parts > 1).then(|| "Baixando vídeo e áudio".to_string());
                    if let (Some(e), Some(DownloadChoice::Video { .. })) = (ext, &j.choice) {
                        j.format_label = e.to_uppercase();
                    }
                });
            }
        }
    }

    fn finalize(self: &Arc<Self>, id: &str, outcome: Outcome) {
        let mut finished_job = None;
        let snapshot = {
            let mut st = self.st();
            st.running.remove(id);
            st.last_emit.remove(id);
            let Some(j) = st.jobs.iter_mut().find(|j| j.id == id) else { return };
            let dest = PathBuf::from(&j.dest_dir);
            j.speed = None;
            j.eta = None;
            match outcome {
                Outcome::Completed(path) => {
                    j.status = JobStatus::Completed;
                    j.stage = None;
                    j.progress = Some(1.0);
                    j.file_size = std::fs::metadata(&path).ok().map(|m| m.len());
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        j.format_label = ext.to_uppercase();
                    }
                    if j.title.is_none() {
                        j.title = path.file_stem().map(|s| s.to_string_lossy().into_owned());
                    }
                    j.file_path = Some(path.display().to_string());
                    j.finished_at = Some(system::now_ms());
                    j.in_history = true;
                    j.error = None;
                    system::cleanup_temp_dir(&dest, id);
                    finished_job = Some(j.clone());
                }
                Outcome::Stopped(StopReason::Pause) => {
                    j.status = JobStatus::Paused;
                    j.stage = self
                        .shutting_down
                        .load(Ordering::SeqCst)
                        .then(|| INTERRUPTED.to_string());
                }
                Outcome::Stopped(StopReason::Cancel) => {
                    j.status = JobStatus::Cancelled;
                    j.stage = None;
                    j.finished_at = Some(system::now_ms());
                    system::cleanup_temp_dir(&dest, id);
                }
                Outcome::Failed(e) => {
                    log::warn!("download {id} falhou: {} ({:?})", e.message, e.detail);
                    j.status = JobStatus::Error;
                    j.stage = None;
                    j.error = Some(e);
                    j.finished_at = Some(system::now_ms());
                    finished_job = Some(j.clone());
                }
            }
            j.clone()
        };

        if snapshot.in_queue {
            self.persist(&snapshot);
            self.sink.job_updated(&snapshot);
        } else {
            // Removido da fila enquanto rodava.
            self.st().jobs.retain(|j| j.id != id);
            if snapshot.in_history {
                self.persist(&snapshot);
            } else {
                let _ = self.db.delete(id);
                system::cleanup_temp_dir(Path::new(&snapshot.dest_dir), id);
            }
            self.sink.job_removed(id);
        }
        if let Some(j) = finished_job {
            self.sink.job_finished(&j);
        }
    }

    // ------------------------------------------------------- ações do usuário

    fn signal(&self, id: &str, reason: StopReason) -> bool {
        match self.st().running.get(id) {
            Some(tx) => {
                let _ = tx.send(Some(reason));
                true
            }
            None => false,
        }
    }

    pub fn cancel(&self, id: &str) -> AppResult<()> {
        if self.signal(id, StopReason::Cancel) {
            return Ok(());
        }
        let job = self
            .update(id, true, |j| {
                if matches!(j.status, JobStatus::Queued | JobStatus::Paused | JobStatus::Error) {
                    j.status = JobStatus::Cancelled;
                    j.stage = None;
                    j.finished_at = Some(system::now_ms());
                }
            })
            .ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        system::cleanup_temp_dir(Path::new(&job.dest_dir), id);
        self.persist(&job);
        Ok(())
    }

    pub fn pause(&self, id: &str) -> AppResult<()> {
        let status = self.get(id).map(|j| j.status).ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        match status {
            JobStatus::Downloading | JobStatus::Analyzing => {
                self.signal(id, StopReason::Pause);
            }
            JobStatus::Queued => {
                if let Some(j) = self.update(id, true, |j| j.status = JobStatus::Paused) {
                    self.persist(&j);
                }
            }
            _ => return Err(AppError::new(ErrorCode::Busy, "Este download não pode ser pausado agora.")),
        }
        Ok(())
    }

    pub fn resume(self: &Arc<Self>, id: &str) -> AppResult<()> {
        let j = self
            .update(id, true, |j| {
                if j.status == JobStatus::Paused {
                    j.status = JobStatus::Queued;
                    j.stage = None;
                    j.error = None;
                }
            })
            .ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        self.persist(&j);
        self.pump();
        Ok(())
    }

    pub fn retry(self: &Arc<Self>, id: &str) -> AppResult<()> {
        let j = self
            .update(id, true, |j| {
                if matches!(j.status, JobStatus::Error | JobStatus::Cancelled) {
                    // Se o formato escolhido sumiu, volta a analisar com a mesma preferência.
                    if j.error.as_ref().map(|e| e.code == ErrorCode::NoFormats).unwrap_or(false) {
                        j.choice = None;
                    }
                    if j.status == JobStatus::Cancelled {
                        j.downloaded_bytes = 0;
                        j.progress = None;
                    }
                    j.status = JobStatus::Queued;
                    j.stage = None;
                    j.error = None;
                    j.finished_at = None;
                }
            })
            .ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        self.persist(&j);
        self.pump();
        Ok(())
    }

    /// Repete um download a partir do histórico, como um novo item da fila.
    pub fn redownload(self: &Arc<Self>, id: &str) -> AppResult<Job> {
        let old = self.get(id).ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        let pref = old.preference.clone().or_else(|| old.choice.as_ref().map(preference_from_choice));
        self.enqueue(&old.url, None, pref, Some(old.dest_dir))
    }

    /// Remove da fila (o histórico de concluídos é preservado).
    pub fn remove(&self, id: &str) -> AppResult<()> {
        let running = self.st().running.contains_key(id);
        if running {
            self.update(id, false, |j| j.in_queue = false);
            self.signal(id, StopReason::Cancel);
            return Ok(());
        }
        let job = {
            let mut st = self.st();
            let pos = st.jobs.iter().position(|j| j.id == id).ok_or_else(|| AppError::not_found("Download não encontrado."))?;
            st.jobs.remove(pos)
        };
        if job.status != JobStatus::Completed {
            system::cleanup_temp_dir(Path::new(&job.dest_dir), id);
        }
        let mut job = job;
        job.in_queue = false;
        if job.in_history {
            self.persist(&job);
        } else {
            self.db.delete(id)?;
        }
        self.sink.job_removed(id);
        Ok(())
    }

    pub fn clear_finished(&self) -> AppResult<usize> {
        let ids: Vec<String> = {
            let st = self.st();
            st.jobs
                .iter()
                .filter(|j| matches!(j.status, JobStatus::Completed | JobStatus::Cancelled) && !st.running.contains_key(&j.id))
                .map(|j| j.id.clone())
                .collect()
        };
        for id in &ids {
            self.remove(id)?;
        }
        Ok(ids.len())
    }

    pub fn history(&self, search: Option<&str>) -> AppResult<Vec<Job>> {
        let mut list = self.db.history(search, 2000)?;
        for j in list.iter_mut() {
            j.file_exists = Some(j.file_path.as_deref().map(|p| Path::new(p).is_file()).unwrap_or(false));
        }
        Ok(list)
    }

    pub fn remove_history(&self, id: &str) -> AppResult<()> {
        if let Some(job) = self.db.remove_from_history(id)? {
            if let Some(j) = self.st().jobs.iter_mut().find(|j| j.id == id) {
                j.in_history = false;
            }
            if let Some(t) = job.thumb_path.filter(|_| !job.in_queue) {
                let _ = std::fs::remove_file(t);
            }
        }
        Ok(())
    }

    pub fn clear_history(&self) -> AppResult<()> {
        let ids = self.db.clear_history()?;
        let mut st = self.st();
        for j in st.jobs.iter_mut() {
            j.in_history = false;
        }
        let queued: Vec<String> = st.jobs.iter().map(|j| j.id.clone()).collect();
        drop(st);
        if let Some(dir) = &self.thumbs_dir {
            for id in ids.iter().filter(|id| !queued.contains(id)) {
                for ext in ["jpg", "png", "webp"] {
                    let _ = std::fs::remove_file(dir.join(format!("{id}.{ext}")));
                }
            }
        }
        Ok(())
    }

    /// Caminho do arquivo concluído de uma tarefa (para abrir ou localizar).
    pub fn file_of(&self, id: &str) -> AppResult<PathBuf> {
        let job = self.get(id).ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        let p = job
            .file_path
            .map(PathBuf::from)
            .ok_or_else(|| AppError::not_found("Este download ainda não gerou um arquivo."))?;
        if !p.is_file() {
            return Err(AppError::not_found("O arquivo não está mais na pasta original. Ele pode ter sido movido ou excluído."));
        }
        Ok(p)
    }

    pub fn folder_of(&self, id: &str) -> AppResult<PathBuf> {
        let job = self.get(id).ok_or_else(|| AppError::not_found("Download não encontrado."))?;
        let p = PathBuf::from(&job.dest_dir);
        if !p.is_dir() {
            return Err(AppError::not_found("A pasta de destino não existe mais."));
        }
        Ok(p)
    }

    /// Chamado ao fechar o app: tarefas em andamento ficam pausadas para retomada.
    pub fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        let mut st = self.st();
        let ids: Vec<String> = st.running.keys().cloned().collect();
        for id in &ids {
            if let Some(tx) = st.running.get(id) {
                let _ = tx.send(Some(StopReason::Pause));
            }
            if let Some(j) = st.jobs.iter_mut().find(|j| &j.id == id) {
                j.status = JobStatus::Paused;
                j.stage = Some(INTERRUPTED.into());
                j.speed = None;
                j.eta = None;
                let _ = self.db.upsert(j);
            }
        }
    }

    // ------------------------------------------------------------ miniaturas

    fn fetch_thumbnail(self: &Arc<Self>, id: String, url: String) {
        let Some(dir) = self.thumbs_dir.clone() else { return };
        if !url.starts_with("https://") {
            return;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let res = async {
                let resp = me.http.get(&url).send().await.ok()?.error_for_status().ok()?;
                let ctype = resp.headers().get("content-type")?.to_str().ok()?.to_string();
                let ext = match ctype.as_str() {
                    t if t.contains("jpeg") || t.contains("jpg") => "jpg",
                    t if t.contains("png") => "png",
                    t if t.contains("webp") => "webp",
                    _ => return None,
                };
                if resp.content_length().unwrap_or(0) > 8 * 1024 * 1024 {
                    return None;
                }
                let bytes = resp.bytes().await.ok()?;
                if bytes.len() > 8 * 1024 * 1024 {
                    return None;
                }
                std::fs::create_dir_all(&dir).ok()?;
                let path = dir.join(format!("{id}.{ext}"));
                std::fs::write(&path, &bytes).ok()?;
                Some(path)
            }
            .await;
            if let Some(path) = res {
                if let Some(j) = me.update(&id, true, |j| j.thumb_path = Some(path.display().to_string())) {
                    me.persist(&j);
                }
            }
        });
    }
}

async fn wait_stop(rx: &mut watch::Receiver<Option<StopReason>>) -> StopReason {
    loop {
        if let Some(r) = *rx.borrow() {
            return r;
        }
        if rx.changed().await.is_err() {
            // Remetente descartado: nunca haverá parada; espera indefinidamente.
            std::future::pending::<()>().await;
        }
    }
}

pub fn preference_from_choice(c: &DownloadChoice) -> Preference {
    match c {
        DownloadChoice::Video { height, container, .. } => Preference::Video {
            max_height: if *height == 0 { u32::MAX } else { *height },
            container: *container,
        },
        DownloadChoice::Audio { format, bitrate, .. } => Preference::Audio { format: *format, bitrate: *bitrate },
    }
}

pub fn estimate_size(info: &MediaInfo, choice: &DownloadChoice) -> Option<u64> {
    match choice {
        DownloadChoice::Video { height, container, video_format_id, .. } => info
            .qualities
            .iter()
            .find(|q| q.height == *height)
            .and_then(|q| {
                q.containers
                    .iter()
                    .find(|c| c.container == *container && Some(&c.video_format_id) == video_format_id.as_ref())
                    .or_else(|| q.containers.iter().find(|c| c.container == *container))
            })
            .and_then(|c| c.size_approx),
        DownloadChoice::Audio { format: AudioFormat::Mp3, bitrate, .. } => {
            info.duration.map(|d| (d * bitrate.unwrap_or(192) as f64 * 1000.0 / 8.0) as u64)
        }
        DownloadChoice::Audio { .. } => info.audio.as_ref().and_then(|a| a.size_approx),
    }
}
