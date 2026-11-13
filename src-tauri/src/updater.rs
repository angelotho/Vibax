//! Atualização do yt-dlp a partir da release oficial no GitHub.
//!
//! Segurança:
//! - somente URLs fixas de github.com/yt-dlp/yt-dlp (HTTPS);
//! - o pacote é validado contra o SHA2-256SUMS publicado na mesma release;
//! - extração com `enclosed_name` (bloqueia caminhos fora da pasta de destino);
//! - a nova versão só é ativada depois de responder a `--version`;
//! - a versão empacotada com o instalador nunca é alterada (serve de fallback).

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::tools::{updates_root, YTDLP_EXE};

const API_LATEST: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const ASSET: &str = "yt-dlp_win.zip";
const MAX_ZIP: u64 = 150 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub updated: bool,
    pub current: Option<String>,
    pub latest: String,
}

/// Compara versões no formato AAAA.MM.DD[.N].
pub fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> { s.trim().split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parse(latest) > parse(current)
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty() && tag.len() <= 20 && tag.chars().all(|c| c.is_ascii_digit() || c == '.') && !tag.contains("..")
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("Vibax-Updater/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(300))
        .https_only(true)
        .build()
        .map_err(|e| AppError::internal("Falha ao preparar a conexão.").with_detail(e.to_string()))
}

fn net_err(e: reqwest::Error) -> AppError {
    AppError::new(ErrorCode::Network, "Não foi possível verificar atualizações. Verifique sua conexão.").with_detail(e.to_string())
}

pub async fn latest_version() -> AppResult<String> {
    let body = client()?
        .get(API_LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(net_err)?
        .error_for_status()
        .map_err(net_err)?
        .text()
        .await
        .map_err(net_err)?;
    let v: serde_json::Value = serde_json::from_str(&body)?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    if !valid_tag(&tag) {
        return Err(AppError::internal("Resposta inesperada do servidor de atualizações."));
    }
    Ok(tag)
}

pub async fn update(local_data: &Path, current: Option<String>) -> AppResult<UpdateResult> {
    let latest = latest_version().await?;
    if let Some(cur) = &current {
        if !is_newer(&latest, cur) {
            return Ok(UpdateResult { updated: false, current, latest });
        }
    }

    let base = format!("https://github.com/yt-dlp/yt-dlp/releases/download/{latest}");
    let http = client()?;

    let sums = http
        .get(format!("{base}/SHA2-256SUMS"))
        .send()
        .await
        .map_err(net_err)?
        .error_for_status()
        .map_err(net_err)?
        .text()
        .await
        .map_err(net_err)?;
    let expected = sums
        .lines()
        .find_map(|l| {
            let mut it = l.split_whitespace();
            let h = it.next()?;
            (it.next()?.trim_start_matches('*') == ASSET).then(|| h.to_ascii_lowercase())
        })
        .ok_or_else(|| AppError::internal("A release não publica o checksum do pacote."))?;

    let resp = http
        .get(format!("{base}/{ASSET}"))
        .send()
        .await
        .map_err(net_err)?
        .error_for_status()
        .map_err(net_err)?;
    if resp.content_length().unwrap_or(0) > MAX_ZIP {
        return Err(AppError::internal("Pacote de atualização maior que o esperado."));
    }
    let mut hasher = Sha256::new();
    let mut data = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(net_err)?;
        hasher.update(&chunk);
        data.extend_from_slice(&chunk);
        if data.len() as u64 > MAX_ZIP {
            return Err(AppError::internal("Pacote de atualização maior que o esperado."));
        }
    }
    let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != expected {
        return Err(AppError::new(
            ErrorCode::Internal,
            "A atualização foi descartada: o arquivo baixado não confere com o checksum oficial.",
        ));
    }

    let root = updates_root(local_data);
    let staging = root.join(format!("staging-{}", uuid::Uuid::new_v4().simple()));
    let final_dir = root.join(format!("yt-dlp-{latest}"));
    std::fs::create_dir_all(&staging)?;
    let result = (|| -> AppResult<()> {
        extract_zip(&data, &staging)?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    let exe = find_exe(&staging).ok_or_else(|| AppError::internal("O pacote não contém o yt-dlp.exe."))?;
    let exe_dir = exe.parent().unwrap_or(&staging).to_path_buf();
    let (ok, out, _) = crate::process::run_capture(&exe, &["--version"], Duration::from_secs(30))
        .await
        .map_err(|e| AppError::internal("A nova versão não pôde ser executada.").with_detail(e.to_string()))?;
    if !ok || out.trim() != latest {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::internal("A nova versão não respondeu corretamente e foi descartada."));
    }

    let _ = std::fs::remove_dir_all(&final_dir);
    std::fs::rename(&exe_dir, &final_dir)?;
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::write(root.join("current.txt"), format!("yt-dlp-{latest}"))?;
    cleanup_old(&root, &format!("yt-dlp-{latest}"));

    Ok(UpdateResult { updated: true, current, latest })
}

fn extract_zip(data: &[u8], dest: &Path) -> AppResult<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data))
        .map_err(|e| AppError::internal("Pacote de atualização inválido.").with_detail(e.to_string()))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AppError::internal("Pacote de atualização inválido.").with_detail(e.to_string()))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err(AppError::internal("Pacote de atualização contém caminhos inválidos."));
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
    }
    Ok(())
}

fn find_exe(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join(YTDLP_EXE);
    if direct.is_file() {
        return Some(direct);
    }
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        if e.path().is_dir() {
            if let Some(p) = find_exe(&e.path()) {
                return Some(p);
            }
        }
    }
    None
}

/// Remove versões antigas (as que estiverem em uso ficam para a próxima vez).
fn cleanup_old(root: &Path, keep: &str) {
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if (name.starts_with("yt-dlp-") || name.starts_with("staging-")) && name != keep {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}

/// Volta a usar a versão que acompanha o instalador.
pub fn reset(local_data: &Path) -> AppResult<()> {
    let root = updates_root(local_data);
    let _ = std::fs::remove_file(root.join("current.txt"));
    cleanup_old(&root, "");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("2026.11.02", "2026.10.26"));
        assert!(is_newer("2026.10.26.1", "2026.10.26"));
        assert!(!is_newer("2026.10.26", "2026.10.26"));
        assert!(!is_newer("2025.12.31", "2026.01.01"));
        assert!(valid_tag("2026.10.26"));
        assert!(!valid_tag("../evil"));
        assert!(!valid_tag(""));
    }
}
