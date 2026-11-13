//! Preferências do usuário, salvas em JSON na pasta de configuração do app.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::ytdlp::info::{AudioFormat, Container};
use crate::ytdlp::spec::MP3_BITRATES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyMode {
    /// Sempre ao concluir ou falhar.
    Always,
    /// Apenas quando a janela do Vibax não estiver em foco.
    Background,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub download_dir: String,
    pub default_kind: DefaultKind,
    pub default_container: Container,
    pub default_audio_format: AudioFormat,
    /// Qualidade máxima preferida (altura nominal). 0 = melhor disponível.
    pub default_quality: u32,
    pub mp3_bitrate: u32,
    pub max_concurrent: u32,
    pub theme: Theme,
    pub notifications: NotifyMode,
    pub clipboard_watch: bool,
    pub embed_metadata: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: String::new(),
            default_kind: DefaultKind::Video,
            default_container: Container::Mp4,
            default_audio_format: AudioFormat::Mp3,
            default_quality: 1080,
            mp3_bitrate: 192,
            max_concurrent: 2,
            theme: Theme::Dark,
            notifications: NotifyMode::Background,
            clipboard_watch: true,
            embed_metadata: true,
        }
    }
}

pub const MAX_CONCURRENT_LIMIT: u32 = 6;
const QUALITIES: &[u32] = &[0, 360, 480, 720, 1080, 1440, 2160, 4320];

impl Settings {
    pub fn normalized(mut self, default_dir: &Path) -> Self {
        if self.download_dir.trim().is_empty() || !Path::new(&self.download_dir).is_absolute() {
            self.download_dir = default_dir.display().to_string();
        }
        self.max_concurrent = self.max_concurrent.clamp(1, MAX_CONCURRENT_LIMIT);
        if !MP3_BITRATES.contains(&self.mp3_bitrate) {
            self.mp3_bitrate = 192;
        }
        if !QUALITIES.contains(&self.default_quality) {
            self.default_quality = 1080;
        }
        self
    }

    pub fn validate(&self) -> AppResult<()> {
        let dir = Path::new(&self.download_dir);
        if !dir.is_absolute() {
            return Err(AppError::new(ErrorCode::Permission, "Escolha uma pasta de downloads válida."));
        }
        if !(1..=MAX_CONCURRENT_LIMIT).contains(&self.max_concurrent) {
            return Err(AppError::internal("Limite de downloads simultâneos inválido."));
        }
        Ok(())
    }

    /// Altura máxima efetiva para a preferência (0 → sem limite).
    pub fn max_height(&self) -> u32 {
        if self.default_quality == 0 {
            u32::MAX
        } else {
            self.default_quality
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    default_dir: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf, default_dir: PathBuf) -> Self {
        Self { path, default_dir }
    }

    pub fn load(&self) -> Settings {
        let parsed = std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default();
        parsed.normalized(&self.default_dir)
    }

    /// Grava de forma atômica (arquivo temporário + rename) para nunca deixar
    /// um JSON pela metade se o app for encerrado durante a escrita.
    pub fn save(&self, settings: &Settings) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_normalizes() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"), dir.path().join("Downloads"));
        let s = store.load();
        assert_eq!(s.download_dir, dir.path().join("Downloads").display().to_string());
        assert_eq!(s.max_concurrent, 2);

        let mut s2 = s.clone();
        s2.max_concurrent = 4;
        s2.theme = Theme::Light;
        s2.default_container = Container::Mkv;
        store.save(&s2).unwrap();
        assert_eq!(store.load(), s2);

        std::fs::write(dir.path().join("settings.json"), r#"{"maxConcurrent": 99, "mp3Bitrate": 7}"#).unwrap();
        let s3 = store.load();
        assert_eq!(s3.max_concurrent, MAX_CONCURRENT_LIMIT);
        assert_eq!(s3.mp3_bitrate, 192);

        std::fs::write(dir.path().join("settings.json"), "{ corrompido").unwrap();
        assert_eq!(store.load().max_concurrent, 2);
    }
}
