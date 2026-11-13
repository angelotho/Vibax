//! Tradução da escolha do usuário (qualidade/formato) em argumentos do yt-dlp.
//!
//! Os argumentos são montados como um vetor — nunca há shell envolvido — e todo
//! valor vindo da interface é validado (IDs de formato com caracteres restritos,
//! bitrate dentro de uma lista fixa, URL validada por `url_guard`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::info::{AudioFormat, Container, MediaInfo};
use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DownloadChoice {
    #[serde(rename_all = "camelCase")]
    Video {
        height: u32,
        label: String,
        container: Container,
        video_format_id: Option<String>,
        audio_format_id: Option<String>,
        #[serde(default)]
        video_has_audio: bool,
    },
    #[serde(rename_all = "camelCase")]
    Audio {
        format: AudioFormat,
        /// kbps para MP3 (128/192/256/320). Ignorado para M4A (cópia sem perdas quando possível).
        bitrate: Option<u32>,
        audio_format_id: Option<String>,
    },
}

/// Preferência genérica, usada quando a mídia ainda não foi analisada
/// (vários links de uma vez, itens de playlist).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Preference {
    #[serde(rename_all = "camelCase")]
    Video { max_height: u32, container: Container },
    #[serde(rename_all = "camelCase")]
    Audio { format: AudioFormat, bitrate: Option<u32> },
}

pub const MP3_BITRATES: &[u32] = &[128, 192, 256, 320];

impl DownloadChoice {
    pub fn validate(&self) -> AppResult<()> {
        let check_id = |id: &Option<String>| -> AppResult<()> {
            if let Some(id) = id {
                let ok = !id.is_empty()
                    && id.len() <= 64
                    && !id.starts_with('-')
                    && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
                if !ok {
                    return Err(AppError::internal("Identificador de formato inválido."));
                }
            }
            Ok(())
        };
        match self {
            DownloadChoice::Video { video_format_id, audio_format_id, height, label, .. } => {
                check_id(video_format_id)?;
                check_id(audio_format_id)?;
                if *height > 8640 || label.len() > 24 || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ') {
                    return Err(AppError::internal("Qualidade inválida."));
                }
            }
            DownloadChoice::Audio { bitrate, audio_format_id, .. } => {
                check_id(audio_format_id)?;
                if let Some(b) = bitrate {
                    if !MP3_BITRATES.contains(b) {
                        return Err(AppError::internal("Taxa de bits inválida."));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn ext_hint(&self) -> &'static str {
        match self {
            DownloadChoice::Video { container, .. } => container.ext(),
            DownloadChoice::Audio { format, .. } => format.ext(),
        }
    }

    pub fn quality_label(&self) -> String {
        match self {
            DownloadChoice::Video { label, .. } => label.clone(),
            DownloadChoice::Audio { format: AudioFormat::Mp3, bitrate, .. } => {
                format!("{} kbps", bitrate.unwrap_or(192))
            }
            DownloadChoice::Audio { .. } => "Original".into(),
        }
    }
}

impl Preference {
    /// Resolve a preferência contra os formatos realmente disponíveis:
    /// escolhe a maior qualidade que não ultrapasse `max_height`, ou a menor
    /// disponível quando todas forem maiores.
    pub fn resolve(&self, info: &MediaInfo) -> AppResult<DownloadChoice> {
        match self {
            Preference::Audio { format, bitrate } => {
                if info.audio.is_none() && info.qualities.iter().all(|q| q.containers.iter().all(|c| !c.video_has_audio)) {
                    return Err(AppError::new(ErrorCode::NoFormats, "Este conteúdo não tem faixa de áudio."));
                }
                Ok(DownloadChoice::Audio {
                    format: *format,
                    bitrate: if *format == AudioFormat::Mp3 { Some(bitrate.unwrap_or(192)) } else { None },
                    audio_format_id: info.audio.as_ref().map(|a| a.format_id.clone()),
                })
            }
            Preference::Video { max_height, container } => {
                let q = info
                    .qualities
                    .iter()
                    .find(|q| q.height <= *max_height) // ordenadas da maior para a menor
                    .or_else(|| info.qualities.last())
                    .ok_or_else(|| AppError::new(ErrorCode::NoFormats, "Este conteúdo não tem fluxo de vídeo."))?;
                let opt = q
                    .containers
                    .iter()
                    .find(|c| c.container == *container)
                    .or_else(|| q.containers.iter().find(|c| c.container == Container::Mp4))
                    .or_else(|| q.containers.first())
                    .ok_or_else(|| AppError::new(ErrorCode::NoFormats, "Formato indisponível."))?;
                Ok(DownloadChoice::Video {
                    height: q.height,
                    label: q.label.clone(),
                    container: opt.container,
                    video_format_id: Some(opt.video_format_id.clone()),
                    audio_format_id: opt.audio_format_id.clone(),
                    video_has_audio: opt.video_has_audio,
                })
            }
        }
    }
}

pub struct ArgsContext<'a> {
    pub url: &'a str,
    pub dest_dir: &'a Path,
    pub temp_dir: &'a Path,
    pub embed_metadata: bool,
}

/// Marcadores das linhas de saída interpretadas por `progress::parse_line`.
pub const TAG_DL: &str = "[vibax-dl]";
pub const TAG_PP: &str = "[vibax-pp]";
pub const TAG_FMT: &str = "[vibax-fmt]";
pub const TAG_FILE: &str = "[vibax-file]";

fn selector(choice: &DownloadChoice) -> String {
    match choice {
        DownloadChoice::Video { container, video_format_id, audio_format_id, video_has_audio, .. } => {
            let mut alts: Vec<String> = Vec::new();
            if let Some(v) = video_format_id {
                if *video_has_audio {
                    alts.push(v.clone());
                } else {
                    if let Some(a) = audio_format_id {
                        alts.push(format!("{v}+{a}"));
                    }
                    match container {
                        Container::Mp4 => alts.push(format!("{v}+ba[ext=m4a]")),
                        Container::Webm => alts.push(format!("{v}+ba[ext=webm]")),
                        Container::Mkv => {}
                    }
                    alts.push(format!("{v}+ba"));
                    alts.push(v.clone());
                }
            }
            // Alternativas genéricas caso os IDs tenham mudado desde a análise
            // (o limite de resolução vem de `-S res:N`).
            match container {
                Container::Mp4 => alts.push("bv*[ext=mp4]+ba[ext=m4a]".into()),
                Container::Webm => alts.push("bv*[ext=webm]+ba[ext=webm]".into()),
                Container::Mkv => {}
            }
            alts.push("bv*+ba".into());
            alts.push("b".into());
            alts.join("/")
        }
        DownloadChoice::Audio { format, audio_format_id, .. } => {
            let mut alts = Vec::new();
            if let Some(a) = audio_format_id {
                alts.push(a.clone());
            }
            if *format == AudioFormat::M4a {
                alts.push("ba[ext=m4a]".into());
            }
            alts.push("ba".into());
            alts.push("b".into());
            alts.join("/")
        }
    }
}

/// Nome do arquivo final: "Título (1080p).mp4" ou "Título.mp3".
/// O limite em bytes evita caminhos longos demais no Windows.
fn output_template(choice: &DownloadChoice) -> String {
    match choice {
        DownloadChoice::Video { label, height, .. } if *height > 0 => format!("%(title).120B ({label}).%(ext)s"),
        _ => "%(title).120B.%(ext)s".into(),
    }
}

pub fn build_download_args(base: Vec<String>, choice: &DownloadChoice, ctx: &ArgsContext) -> Vec<String> {
    let mut a = base;
    let push = |a: &mut Vec<String>, items: &[&str]| a.extend(items.iter().map(|s| s.to_string()));

    push(&mut a, &["--no-playlist", "--newline", "--progress", "--no-simulate", "--no-mtime"]);
    push(&mut a, &["--retries", "10", "--fragment-retries", "10", "--concurrent-fragments", "4"]);
    push(&mut a, &["--continue", "--part", "--windows-filenames"]);

    a.push("-f".into());
    a.push(selector(choice));

    match choice {
        DownloadChoice::Video { container, height, .. } => {
            if *height > 0 {
                a.push("-S".into());
                a.push(format!("res:{height}"));
            }
            match container {
                Container::Mp4 => push(&mut a, &["--merge-output-format", "mp4", "--remux-video", "mp4"]),
                // WebM só aceita VP8/VP9/AV1 + Opus/Vorbis; se a fonte mudou, cai para MKV.
                Container::Webm => push(&mut a, &["--merge-output-format", "webm/mkv"]),
                Container::Mkv => push(&mut a, &["--merge-output-format", "mkv", "--remux-video", "mkv"]),
            }
            if ctx.embed_metadata {
                push(&mut a, &["--embed-metadata", "--embed-chapters"]);
                if *container != Container::Webm {
                    a.push("--embed-thumbnail".into());
                }
            }
        }
        DownloadChoice::Audio { format, bitrate, .. } => {
            push(&mut a, &["-x", "--audio-format", format.ext()]);
            match format {
                AudioFormat::Mp3 => {
                    a.push("--audio-quality".into());
                    a.push(format!("{}K", bitrate.unwrap_or(192)));
                }
                AudioFormat::M4a => push(&mut a, &["--audio-quality", "192K"]),
            }
            if ctx.embed_metadata {
                push(&mut a, &["--embed-metadata", "--embed-thumbnail"]);
            }
        }
    }

    a.push("-P".into());
    a.push(format!("home:{}", ctx.dest_dir.display()));
    a.push("-P".into());
    a.push(format!("temp:{}", ctx.temp_dir.display()));
    a.push("-o".into());
    a.push(output_template(choice));

    a.push("--progress-template".into());
    a.push(format!(
        "download:{TAG_DL}%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s"
    ));
    a.push("--progress-template".into());
    a.push(format!("postprocess:{TAG_PP}%(progress.status)s|%(progress.postprocessor)s"));
    a.push("--print".into());
    a.push(format!("before_dl:{TAG_FMT}%(format_id)s|%(ext)s|%(filesize,filesize_approx)s"));
    a.push("--print".into());
    a.push(format!("after_move:{TAG_FILE}%(filepath)s"));

    // "--" encerra as opções: a URL nunca é interpretada como argumento.
    a.push("--".into());
    a.push(ctx.url.to_string());
    a
}

pub fn build_probe_args(base: Vec<String>, url: &str) -> Vec<String> {
    let mut a = base;
    a.extend(["-J", "--no-playlist", "--flat-playlist", "--no-warnings", "--"].map(String::from));
    a.push(url.to_string());
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx<'a>(d: &'a Path, t: &'a Path) -> ArgsContext<'a> {
        ArgsContext { url: "https://www.youtube.com/watch?v=abc", dest_dir: d, temp_dir: t, embed_metadata: true }
    }

    #[test]
    fn video_mp4_args() {
        let choice = DownloadChoice::Video {
            height: 1080,
            label: "1080p60".into(),
            container: Container::Mp4,
            video_format_id: Some("299".into()),
            audio_format_id: Some("140".into()),
            video_has_audio: false,
        };
        choice.validate().unwrap();
        let d = PathBuf::from("D:\\Vídeos");
        let t = PathBuf::from("D:\\Vídeos\\.vibax-tmp\\1");
        let a = build_download_args(vec![], &choice, &ctx(&d, &t));
        let f = a.iter().position(|x| x == "-f").unwrap();
        assert_eq!(a[f + 1], "299+140/299+ba[ext=m4a]/299+ba/299/bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b");
        assert!(a.windows(2).any(|w| w == ["-S", "res:1080"]));
        assert!(a.windows(2).any(|w| w == ["--merge-output-format", "mp4"]));
        assert!(a.contains(&"home:D:\\Vídeos".to_string()));
        assert!(a.contains(&"%(title).120B (1080p60).%(ext)s".to_string()));
        assert_eq!(&a[a.len() - 2..], ["--", "https://www.youtube.com/watch?v=abc"]);
        assert!(!a.iter().any(|x| x.contains("cookies")));
    }

    #[test]
    fn audio_mp3_args() {
        let choice = DownloadChoice::Audio { format: AudioFormat::Mp3, bitrate: Some(320), audio_format_id: Some("251".into()) };
        let d = PathBuf::from("C:\\m");
        let a = build_download_args(vec![], &choice, &ctx(&d, &d));
        assert!(a.windows(3).any(|w| w == ["-x", "--audio-format", "mp3"]));
        assert!(a.windows(2).any(|w| w == ["--audio-quality", "320K"]));
        assert!(a.contains(&"251/ba/b".to_string()));
    }

    #[test]
    fn rejects_injection_in_ids() {
        for bad in ["--exec", "1+2", "a/b", "x y", "137[height>1]"] {
            let c = DownloadChoice::Video {
                height: 720,
                label: "720p".into(),
                container: Container::Mp4,
                video_format_id: Some(bad.into()),
                audio_format_id: None,
                video_has_audio: true,
            };
            assert!(c.validate().is_err(), "{bad}");
        }
        let c = DownloadChoice::Audio { format: AudioFormat::Mp3, bitrate: Some(999), audio_format_id: None };
        assert!(c.validate().is_err());
    }
}
