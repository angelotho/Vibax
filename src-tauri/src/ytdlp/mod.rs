//! Integração com o yt-dlp: análise de links e execução de downloads.

pub mod classify;
pub mod info;
pub mod progress;
pub mod spec;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::process::{self, ProcessGroup};
use crate::tools::ToolPaths;
use info::MediaInfo;
use progress::{Line, Tracker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Cancel,
    Pause,
}

#[derive(Debug)]
pub enum RunError {
    Stopped(StopReason),
    Failed(AppError),
}

/// Atualização de progresso enviada durante o download.
#[derive(Debug, Clone)]
pub enum Update {
    Progress { fraction: Option<f64>, downloaded: u64, total: Option<u64>, speed: Option<f64>, eta: Option<u64> },
    Stage(&'static str),
    Format { ext: Option<String>, size: Option<u64>, parts: usize },
}

const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Engine {
    pub tools: Arc<ToolPaths>,
}

impl Engine {
    pub fn new(tools: Arc<ToolPaths>) -> Self {
        Self { tools }
    }

    /// Analisa um link e devolve título, miniatura, duração e formatos reais.
    pub async fn probe(&self, url: &str) -> AppResult<MediaInfo> {
        let url = crate::url_guard::validate(url)?;
        let args = spec::build_probe_args(self.tools.base_args(), &url);
        let child = process::command(&self.tools.ytdlp, &args).spawn().map_err(|e| {
            AppError::new(ErrorCode::ToolMissing, "Não foi possível iniciar o mecanismo de download.").with_detail(e.to_string())
        })?;
        let group = ProcessGroup::new().map_err(AppError::from)?;
        let _ = group.assign(&child);

        let output = match tokio::time::timeout(PROBE_TIMEOUT, child.wait_with_output()).await {
            Ok(out) => out.map_err(AppError::from)?,
            Err(_) => {
                group.terminate();
                return Err(AppError::new(
                    ErrorCode::Network,
                    "A análise demorou demais. Verifique sua conexão e tente novamente.",
                ));
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() || stdout.trim().is_empty() {
            let errors: Vec<String> = stderr
                .lines()
                .filter_map(|l| l.strip_prefix("ERROR:").map(|s| s.trim().to_string()))
                .collect();
            return Err(classify::classify(&errors, &stderr));
        }
        let json: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
            AppError::internal("Resposta inesperada ao analisar o link.").with_detail(e.to_string())
        })?;
        info::parse_info(&json, &url)
    }

    /// Executa um download. `on_update` é chamado para cada evento relevante.
    pub async fn download(
        &self,
        args: Vec<String>,
        mut stop: watch::Receiver<Option<StopReason>>,
        mut on_update: impl FnMut(Update) + Send,
    ) -> Result<PathBuf, RunError> {
        let mut child = process::command(&self.tools.ytdlp, &args).spawn().map_err(|e| {
            RunError::Failed(
                AppError::new(ErrorCode::ToolMissing, "Não foi possível iniciar o mecanismo de download.")
                    .with_detail(e.to_string()),
            )
        })?;
        let group = ProcessGroup::new().map_err(|e| RunError::Failed(e.into()))?;
        if let Err(e) = group.assign(&child) {
            log::warn!("falha ao associar processo ao Job Object: {e}");
        }

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        // O yt-dlp escreve o progresso de download no stdout e o de pós-processamento
        // (e os erros) no stderr: os dois fluxos são lidos em paralelo para um canal único.
        let (tx, mut lines) = tokio::sync::mpsc::unbounded_channel::<(bool, String)>();
        let readers = [
            spawn_line_reader(stdout, false, tx.clone()),
            spawn_line_reader(stderr, true, tx),
        ];

        let mut errors: Vec<String> = Vec::new();
        let mut tail: Vec<String> = Vec::new();
        let mut tracker = Tracker::default();
        let mut final_path: Option<PathBuf> = None;
        let mut stop_reason: Option<StopReason> = *stop.borrow();
        let mut stop_closed = false;
        if stop_reason.is_some() {
            group.terminate();
            let _ = child.start_kill();
        }

        while stop_reason.is_none() {
            tokio::select! {
                changed = stop.changed(), if !stop_closed => {
                    if changed.is_err() {
                        stop_closed = true;
                    } else if let Some(r) = *stop.borrow() {
                        stop_reason = Some(r);
                        group.terminate();
                        let _ = child.start_kill();
                    }
                }
                msg = lines.recv() => {
                    let Some((from_stderr, line)) = msg else { break };
                    if from_stderr && !line.starts_with('[') {
                        tail.push(line.clone());
                        if tail.len() > 40 {
                            tail.remove(0);
                        }
                    }
                    match progress::parse_line(&line) {
                        Line::Format { format_id, ext, size } => {
                            tracker.on_format(&format_id, &ext, size);
                            on_update(Update::Format { ext: tracker.ext.clone(), size, parts: tracker.parts });
                        }
                        Line::Download { finished, downloaded, total, speed, eta } => {
                            tracker.on_download(finished, downloaded, total, speed, eta);
                            on_update(Update::Progress {
                                fraction: tracker.fraction(),
                                downloaded: tracker.downloaded,
                                total: tracker.total(),
                                speed: tracker.speed,
                                eta: tracker.eta,
                            });
                        }
                        Line::PostProcess { started: true, name } => on_update(Update::Stage(progress::stage_label(&name))),
                        Line::PostProcess { .. } => {}
                        Line::File(p) => final_path = Some(PathBuf::from(p)),
                        Line::Error(msg) => errors.push(msg),
                        Line::Other => {}
                    }
                }
            }
        }

        let status = child.wait().await;
        for r in readers {
            let _ = tokio::time::timeout(Duration::from_secs(2), r).await;
        }
        // Linhas que chegaram depois do fim do laço (ex.: o último ERROR).
        while let Ok((_, line)) = lines.try_recv() {
            match progress::parse_line(&line) {
                Line::Error(msg) => errors.push(msg),
                Line::File(p) => final_path = Some(PathBuf::from(p)),
                _ => {}
            }
        }

        if let Some(r) = stop_reason.or(*stop.borrow()) {
            return Err(RunError::Stopped(r));
        }

        let ok = status.map(|s| s.success()).unwrap_or(false);
        match (ok, final_path) {
            (true, Some(p)) if p.is_file() => Ok(p),
            (true, Some(p)) => Err(RunError::Failed(
                AppError::new(ErrorCode::Processing, "O download terminou, mas o arquivo final não foi encontrado.")
                    .with_detail(p.display().to_string()),
            )),
            (true, None) => Err(RunError::Failed(AppError::new(
                ErrorCode::Processing,
                "O download terminou sem gerar um arquivo.",
            ))),
            (false, _) => Err(RunError::Failed(classify::classify(&errors, &tail.join("\n")))),
        }
    }
}

fn spawn_line_reader(
    stream: impl AsyncRead + Unpin + Send + 'static,
    from_stderr: bool,
    tx: UnboundedSender<(bool, String)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                    if !line.is_empty() && tx.send((from_stderr, line)).is_err() {
                        break;
                    }
                }
            }
        }
    })
}
