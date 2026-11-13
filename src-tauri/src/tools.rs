//! Localização dos componentes externos (yt-dlp, FFmpeg, Deno).
//!
//! Ordem de busca:
//! 1. `VIBAX_TOOLS_DIR` (apenas para testes/diagnóstico);
//! 2. yt-dlp atualizado pelo próprio Vibax em `%LOCALAPPDATA%\<app>\tools\` (somente yt-dlp);
//! 3. pasta `tools` empacotada como recurso do aplicativo (instalação);
//! 4. em builds de desenvolvimento, `src-tauri/tools`.
//!
//! Nunca são usados executáveis do PATH global: o app deve funcionar igual na
//! máquina do desenvolvedor e na do usuário final.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::process;

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    /// Pasta que contém ffmpeg.exe e ffprobe.exe.
    pub ffmpeg_dir: PathBuf,
    pub deno: PathBuf,
    pub ytdlp_updated: bool,
    /// Cache do yt-dlp (acelera a resolução de desafios do YouTube entre execuções).
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub ytdlp_version: Option<String>,
    pub ytdlp_path: String,
    pub ytdlp_updated: bool,
    pub ffmpeg_version: Option<String>,
    pub deno_version: Option<String>,
}

pub const YTDLP_EXE: &str = "yt-dlp.exe";
pub const FFMPEG_EXE: &str = "ffmpeg.exe";
pub const FFPROBE_EXE: &str = "ffprobe.exe";
pub const DENO_EXE: &str = "deno.exe";

/// Diretório onde atualizações do yt-dlp são instaladas.
pub fn updates_root(local_data: &Path) -> PathBuf {
    local_data.join("tools")
}

/// Pasta da versão atualizada do yt-dlp em uso, se houver uma válida.
pub fn updated_ytdlp_dir(local_data: &Path) -> Option<PathBuf> {
    let root = updates_root(local_data);
    let name = std::fs::read_to_string(root.join("current.txt")).ok()?;
    let name = name.trim();
    // O nome é gerado pelo próprio app (yt-dlp-AAAA.MM.DD); rejeita qualquer outra coisa.
    let version = name.strip_prefix("yt-dlp-")?;
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') || version.contains("..") {
        return None;
    }
    let dir = root.join(name);
    dir.join(YTDLP_EXE).is_file().then_some(dir)
}

fn bundled_candidates(resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(dir) = std::env::var("VIBAX_TOOLS_DIR") {
        v.push(PathBuf::from(dir));
    }
    if let Some(r) = resource_dir {
        v.push(r.join("tools"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            v.push(parent.join("tools"));
        }
    }
    if cfg!(debug_assertions) {
        v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools"));
    }
    v
}

pub fn resolve(resource_dir: Option<&Path>, local_data: &Path, cache_dir: PathBuf) -> AppResult<ToolPaths> {
    let bundled = bundled_candidates(resource_dir)
        .into_iter()
        .find(|d| d.join(FFMPEG_EXE).is_file() && d.join("yt-dlp").join(YTDLP_EXE).is_file())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::ToolMissing,
                "Os componentes de download não foram encontrados. Reinstale o Vibax.",
            )
        })?;

    let (ytdlp, updated) = match updated_ytdlp_dir(local_data) {
        Some(dir) if std::env::var("VIBAX_TOOLS_DIR").is_err() => (dir.join(YTDLP_EXE), true),
        _ => (bundled.join("yt-dlp").join(YTDLP_EXE), false),
    };

    let deno = bundled.join(DENO_EXE);
    if !deno.is_file() {
        return Err(AppError::new(
            ErrorCode::ToolMissing,
            "O runtime JavaScript (Deno) não foi encontrado. Reinstale o Vibax.",
        ));
    }
    if !bundled.join(FFPROBE_EXE).is_file() {
        return Err(AppError::new(ErrorCode::ToolMissing, "O FFprobe não foi encontrado. Reinstale o Vibax."));
    }

    Ok(ToolPaths { ytdlp, ffmpeg_dir: bundled.clone(), deno, ytdlp_updated: updated, cache_dir })
}

impl ToolPaths {
    /// Argumentos comuns a toda chamada do yt-dlp.
    ///
    /// `--ignore-config` e `--no-plugin-dirs` impedem que arquivos de configuração
    /// ou plugins instalados na máquina alterem o comportamento (ex.: injetar
    /// cookies). Nenhuma opção de cookies/credenciais é usada pelo Vibax.
    pub fn base_args(&self) -> Vec<String> {
        vec![
            "--ignore-config".into(),
            "--no-plugin-dirs".into(),
            "--no-colors".into(),
            "--encoding".into(),
            "utf-8".into(),
            "--js-runtimes".into(),
            format!("deno:{}", self.deno.display()),
            "--ffmpeg-location".into(),
            self.ffmpeg_dir.display().to_string(),
            "--cache-dir".into(),
            self.cache_dir.display().to_string(),
            "--socket-timeout".into(),
            "20".into(),
        ]
    }

    pub async fn tools_info_version(&self) -> Option<String> {
        process::run_capture(&self.ytdlp, &["--version"], Duration::from_secs(30))
            .await
            .ok()
            .and_then(|(_, o, _)| o.lines().next().map(|l| l.trim().to_string()))
    }

    pub async fn info(&self) -> ToolInfo {
        let t = Duration::from_secs(20);
        let first_line = |s: String| s.lines().next().map(|l| l.trim().to_string());
        let ytdlp_version = process::run_capture(&self.ytdlp, &["--version"], t)
            .await
            .ok()
            .and_then(|(_, o, _)| first_line(o));
        let ffmpeg_version = process::run_capture(&self.ffmpeg_dir.join(FFMPEG_EXE), &["-hide_banner", "-version"], t)
            .await
            .ok()
            .and_then(|(_, o, _)| first_line(o))
            .map(|l| l.trim_start_matches("ffmpeg version ").split(" Copyright").next().unwrap_or("").to_string());
        let deno_version = process::run_capture(&self.deno, &["--version"], t)
            .await
            .ok()
            .and_then(|(_, o, _)| first_line(o));
        ToolInfo {
            ytdlp_version,
            ytdlp_path: self.ytdlp.display().to_string(),
            ytdlp_updated: self.ytdlp_updated,
            ffmpeg_version,
            deno_version,
        }
    }
}
