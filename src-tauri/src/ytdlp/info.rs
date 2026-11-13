//! Interpretação do JSON do yt-dlp (`-J`) em um modelo enxuto para a interface.
//!
//! As opções de qualidade são derivadas exclusivamente dos formatos que a fonte
//! anuncia: nenhuma resolução é oferecida se não existir um fluxo correspondente.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Mp4,
    Webm,
    Mkv,
}

impl Container {
    pub fn ext(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Webm => "webm",
            Container::Mkv => "mkv",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
}

impl AudioFormat {
    pub fn ext(self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerOption {
    pub container: Container,
    pub video_format_id: String,
    pub audio_format_id: Option<String>,
    /// O fluxo de vídeo escolhido já contém áudio (não precisa mesclar).
    pub video_has_audio: bool,
    pub vcodec: String,
    pub size_approx: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QualityOption {
    /// Altura nominal (360, 720, 1080, 2160…), calculada pelo lado menor do quadro.
    pub height: u32,
    pub label: String,
    pub fps: Option<u32>,
    pub containers: Vec<ContainerOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioOption {
    pub format_id: String,
    pub acodec: String,
    pub abr: Option<f64>,
    pub size_approx: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub url: String,
    pub title: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub kind: MediaKind,
    pub id: String,
    pub url: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub source: String,
    pub source_label: String,
    pub qualities: Vec<QualityOption>,
    pub audio: Option<AudioOption>,
    pub entries: Vec<PlaylistEntry>,
}

/// Degraus de resolução: (altura nominal, largura nominal 16:9).
const LADDER: &[(u32, u32)] = &[
    (144, 256),
    (240, 426),
    (360, 640),
    (480, 854),
    (720, 1280),
    (1080, 1920),
    (1440, 2560),
    (2160, 3840),
    (4320, 7680),
];

/// Classifica um quadro WxH no degrau nominal (ex.: 1920x800 → 1080p; 1080x1920 → 1080p).
pub fn nominal_height(width: Option<u32>, height: Option<u32>) -> Option<u32> {
    let (w, h) = match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w.max(h), w.min(h)),
        (None, Some(h)) if h > 0 => (h * 16 / 9, h),
        (Some(w), None) if w > 0 => (w, w * 9 / 16),
        _ => return None,
    };
    let mut best = None;
    for &(lh, lw) in LADDER {
        // Tolerância de 10% para codificações com bordas recortadas.
        if h * 10 >= lh * 9 || w * 10 >= lw * 9 {
            best = Some(lh);
        }
    }
    best.or(Some(h))
}

pub fn quality_label(height: u32, fps: Option<u32>) -> String {
    let base = match height {
        2160 => "4K".to_string(),
        4320 => "8K".to_string(),
        h => format!("{h}p"),
    };
    match fps {
        Some(f) if f >= 48 && height < 2160 => format!("{base}{f}"),
        Some(f) if f >= 48 => format!("{base} {f}fps"),
        _ => base,
    }
}

fn friendly_vcodec(v: &str) -> String {
    let v = v.to_ascii_lowercase();
    if v.starts_with("avc") || v.starts_with("h264") {
        "H.264"
    } else if v.starts_with("av01") || v == "av1" {
        "AV1"
    } else if v.starts_with("vp09") || v.starts_with("vp9") {
        "VP9"
    } else if v.starts_with("hev") || v.starts_with("hvc") || v.starts_with("h265") {
        "H.265"
    } else if v.starts_with("vp8") {
        "VP8"
    } else {
        return v.split('.').next().unwrap_or("").to_uppercase();
    }
    .to_string()
}

pub fn friendly_source(extractor_key: &str) -> String {
    match extractor_key.to_ascii_lowercase().as_str() {
        "youtube" | "youtubetab" => "YouTube".into(),
        "vimeo" => "Vimeo".into(),
        "generic" => "Link direto".into(),
        "dailymotion" => "Dailymotion".into(),
        "twitch" | "twitchvod" | "twitchclips" => "Twitch".into(),
        "soundcloud" => "SoundCloud".into(),
        "bandcamp" => "Bandcamp".into(),
        "archiveorg" => "Internet Archive".into(),
        "wikimedia" | "commonsmediawiki" => "Wikimedia Commons".into(),
        "peertube" => "PeerTube".into(),
        other if !other.is_empty() => {
            let mut c = extractor_key.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
        _ => "Web".into(),
    }
}

/// Visão normalizada de um formato do yt-dlp.
#[derive(Debug, Clone)]
struct Fmt {
    id: String,
    ext: String,
    vcodec: Option<String>,
    acodec: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    tbr: Option<f64>,
    abr: Option<f64>,
    size: Option<u64>,
    protocol: String,
    note: String,
    channels: Option<u32>,
}

fn as_str(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}
fn as_f64(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}
fn as_u32(v: &Value, k: &str) -> Option<u32> {
    v.get(k).and_then(Value::as_f64).filter(|x| *x > 0.0).map(|x| x as u32)
}

fn codec(v: Option<String>) -> Option<String> {
    v.filter(|c| !c.is_empty() && c != "none")
}

fn parse_formats(root: &Value, duration: Option<f64>) -> Vec<Fmt> {
    let Some(list) = root.get("formats").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|f| {
            let id = as_str(f, "format_id")?;
            let protocol = as_str(f, "protocol").unwrap_or_default();
            // Storyboards / imagens e manifestos não baixáveis.
            if protocol == "mhtml" || as_str(f, "ext").as_deref() == Some("mhtml") {
                return None;
            }
            if f.get("has_drm").and_then(Value::as_bool).unwrap_or(false) {
                return None;
            }
            let mut vcodec = codec(as_str(f, "vcodec"));
            let mut acodec = codec(as_str(f, "acodec"));
            let width = as_u32(f, "width");
            let height = as_u32(f, "height");
            // Fontes genéricas às vezes não informam codecs.
            if vcodec.is_none() && acodec.is_none() {
                if height.is_some() || width.is_some() {
                    vcodec = Some("unknown".into());
                    acodec = as_str(f, "acodec").filter(|a| a != "none").or(Some("unknown".into()));
                } else if as_str(f, "vcodec").as_deref() == Some("none")
                    || as_str(f, "resolution").as_deref() == Some("audio only")
                    || matches!(as_str(f, "ext").as_deref(), Some("m4a" | "mp3" | "opus" | "ogg" | "oga" | "wav" | "flac" | "aac" | "weba"))
                {
                    acodec = Some("unknown".into());
                } else {
                    vcodec = Some("unknown".into());
                    acodec = Some("unknown".into());
                }
            }
            let tbr = as_f64(f, "tbr");
            let size = f
                .get("filesize")
                .and_then(Value::as_f64)
                .or_else(|| f.get("filesize_approx").and_then(Value::as_f64))
                .or_else(|| match (tbr, duration) {
                    (Some(t), Some(d)) => Some(t * 1000.0 / 8.0 * d),
                    _ => None,
                })
                .map(|x| x as u64);
            Some(Fmt {
                ext: as_str(f, "ext").unwrap_or_default(),
                vcodec,
                acodec,
                width,
                height,
                fps: as_f64(f, "fps"),
                tbr,
                abr: as_f64(f, "abr"),
                size,
                protocol,
                note: as_str(f, "format_note").unwrap_or_default().to_ascii_lowercase(),
                channels: as_u32(f, "audio_channels"),
                id,
            })
        })
        .collect()
}

fn vcodec_family(f: &Fmt) -> &str {
    let v = f.vcodec.as_deref().unwrap_or("");
    if v.starts_with("avc") || v.starts_with("h264") {
        "avc"
    } else if v.starts_with("av01") {
        "av1"
    } else if v.starts_with("vp09") || v.starts_with("vp9") {
        "vp9"
    } else if v.starts_with("hev") || v.starts_with("hvc") {
        "hevc"
    } else if v.starts_with("vp8") {
        "vp8"
    } else {
        "other"
    }
}

fn is_webm_audio(f: &Fmt) -> bool {
    let a = f.acodec.as_deref().unwrap_or("");
    a.starts_with("opus") || a.starts_with("vorbis")
}

fn is_m4a_audio(f: &Fmt) -> bool {
    let a = f.acodec.as_deref().unwrap_or("");
    a.starts_with("mp4a") || a == "aac" || f.ext == "m4a"
}

/// Penaliza faixas de áudio "DRC", dubladas ou de codec desconhecido.
fn audio_quality_score(f: &Fmt) -> f64 {
    let mut s = f.abr.or(f.tbr).unwrap_or(64.0);
    if f.note.contains("drc") || f.id.contains("drc") {
        s -= 1000.0;
    }
    if f.acodec.as_deref().map(|a| a == "unknown" || a.starts_with("iamf")).unwrap_or(true) {
        s -= 2000.0;
    }
    if f.note.contains("dubbed") || f.note.contains("descriptive") {
        s -= 500.0;
    }
    if f.protocol.contains("m3u8") {
        s -= 10.0;
    }
    // Estéreo é o padrão: áudio surround (5.1) tem compatibilidade pior em celulares e TVs.
    if f.channels.map(|c| c > 2).unwrap_or(false) {
        s -= 400.0;
    }
    s
}

fn best_audio<'a>(fmts: &'a [Fmt], pred: impl Fn(&Fmt) -> bool) -> Option<&'a Fmt> {
    fmts.iter()
        .filter(|f| f.vcodec.is_none() && f.acodec.is_some() && pred(f))
        .max_by(|a, b| audio_quality_score(a).total_cmp(&audio_quality_score(b)))
}

fn protocol_score(f: &Fmt) -> i32 {
    if f.protocol.starts_with("http") && !f.protocol.contains("m3u8") {
        2
    } else if f.protocol.contains("dash") {
        1
    } else {
        0
    }
}

pub fn parse_info(root: &Value, original_url: &str) -> AppResult<MediaInfo> {
    let source = as_str(root, "extractor_key").unwrap_or_else(|| "Generic".into());
    let source_label = friendly_source(&source);
    let title = as_str(root, "title").filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "Sem título".into());
    let id = as_str(root, "id").unwrap_or_default();
    let url = as_str(root, "webpage_url").unwrap_or_else(|| original_url.to_string());
    let uploader = as_str(root, "uploader").or_else(|| as_str(root, "channel"));
    let thumbnail = as_str(root, "thumbnail").or_else(|| {
        root.get("thumbnails")
            .and_then(Value::as_array)
            .and_then(|t| t.iter().rev().find_map(|x| as_str(x, "url")))
    });

    let typ = as_str(root, "_type").unwrap_or_else(|| "video".into());
    if typ == "playlist" {
        let entries: Vec<PlaylistEntry> = root
            .get("entries")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|e| {
                        let u = as_str(e, "url").or_else(|| as_str(e, "webpage_url"))?;
                        crate::url_guard::validate(&u).ok().map(|url| PlaylistEntry {
                            url,
                            title: as_str(e, "title"),
                            duration: as_f64(e, "duration"),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if entries.is_empty() {
            return Err(AppError::new(ErrorCode::NoFormats, "Esta playlist não tem vídeos disponíveis."));
        }
        return Ok(MediaInfo {
            kind: MediaKind::Playlist,
            id,
            url,
            title,
            uploader,
            duration: None,
            thumbnail,
            source,
            source_label,
            qualities: Vec::new(),
            audio: None,
            entries,
        });
    }

    if root.get("is_live").and_then(Value::as_bool).unwrap_or(false)
        || as_str(root, "live_status").as_deref() == Some("is_upcoming")
    {
        return Err(AppError::new(
            ErrorCode::Live,
            "Transmissões ao vivo ou agendadas não são suportadas. Tente novamente quando o vídeo estiver disponível por completo.",
        ));
    }

    let duration = as_f64(root, "duration");
    let mut fmts = parse_formats(root, duration);
    if fmts.is_empty() {
        // Algumas fontes retornam um único arquivo sem lista de formatos.
        if let Some(fid) = as_str(root, "format_id").or_else(|| as_str(root, "url").map(|_| "0".into())) {
            let mut single = root.clone();
            single["format_id"] = Value::String(fid);
            fmts = parse_formats(&serde_json::json!({ "formats": [single] }), duration);
        }
    }
    if fmts.is_empty() {
        let drm = root.get("formats").and_then(Value::as_array).map(|l| !l.is_empty()).unwrap_or(false);
        return Err(if drm {
            AppError::new(ErrorCode::Drm, "Este conteúdo é protegido (DRM) e não pode ser baixado.")
        } else {
            AppError::new(ErrorCode::NoFormats, "A fonte não oferece nenhum arquivo para download.")
        });
    }

    let audio_any = best_audio(&fmts, |_| true);
    let audio_m4a = best_audio(&fmts, is_m4a_audio);
    let audio_webm = best_audio(&fmts, is_webm_audio);

    // Agrupa vídeos por altura nominal.
    let mut buckets: std::collections::BTreeMap<u32, Vec<&Fmt>> = Default::default();
    for f in fmts.iter().filter(|f| f.vcodec.is_some()) {
        if let Some(h) = nominal_height(f.width, f.height) {
            buckets.entry(h).or_default().push(f);
        } else if f.vcodec.as_deref() == Some("unknown") && !fmts.iter().any(|g| nominal_height(g.width, g.height).is_some()) {
            // Arquivo único de resolução desconhecida (ex.: link direto). Usa 0 = "Original".
            buckets.entry(0).or_default().push(f);
        }
    }

    let mut qualities = Vec::new();
    for (height, list) in buckets.iter().rev() {
        let max_fps = list.iter().filter_map(|f| f.fps).fold(0.0_f64, f64::max);
        // Mantém apenas os formatos com a maior taxa de quadros do degrau.
        let top: Vec<&Fmt> = list
            .iter()
            .copied()
            .filter(|f| max_fps == 0.0 || f.fps.unwrap_or(0.0) >= max_fps - 1.0)
            .collect();
        let fps = (max_fps > 0.0).then(|| max_fps.round() as u32);

        // Quando a fonte oferece áudio separado, prefere vídeo "puro" + melhor áudio:
        // o áudio sai em qualidade superior e evita fluxos progressivos legados
        // (ex.: itag 18 do YouTube), que hoje costumam ser recusados com HTTP 403.
        let separate = |f: &Fmt| (audio_any.is_some() && f.acodec.is_none()) as i32;
        let pick = |rank: &dyn Fn(&Fmt) -> Option<i32>| -> Option<&Fmt> {
            top.iter()
                .copied()
                .filter_map(|f| rank(f).map(|r| (r, f)))
                .max_by(|(ra, a), (rb, b)| {
                    ra.cmp(rb)
                        .then(separate(a).cmp(&separate(b)))
                        .then(protocol_score(a).cmp(&protocol_score(b)))
                        .then(a.tbr.unwrap_or(0.0).total_cmp(&b.tbr.unwrap_or(0.0)))
                })
                .map(|(_, f)| f)
        };

        let mut containers = Vec::new();
        let mut push = |container: Container, v: &Fmt, audio: Option<&Fmt>| {
            let has_audio = v.acodec.is_some();
            let audio = if has_audio { None } else { audio };
            if !has_audio && audio.is_none() && audio_any.is_some() {
                return;
            }
            let size = match (v.size, audio.and_then(|a| a.size)) {
                (Some(a), Some(b)) => Some(a + b),
                (Some(a), None) if audio.is_none() => Some(a),
                (s, _) => s,
            };
            containers.push(ContainerOption {
                container,
                video_format_id: v.id.clone(),
                audio_format_id: audio.map(|a| a.id.clone()),
                video_has_audio: has_audio,
                vcodec: v.vcodec.as_deref().map(friendly_vcodec).unwrap_or_default(),
                size_approx: size,
            });
        };

        // MP4: H.264 primeiro (máxima compatibilidade), depois HEVC, AV1 e por fim
        // VP9 (que será remultiplexado em MP4 sem recodificação).
        if let Some(v) = pick(&|f| match vcodec_family(f) {
            "avc" => Some(4),
            "hevc" => Some(3),
            "av1" => Some(2),
            "vp9" => Some(1),
            "other" if f.ext == "mp4" => Some(1),
            _ => None,
        }) {
            push(Container::Mp4, v, audio_m4a.or(audio_any));
        }
        // WebM: apenas quando há vídeo VP9/AV1/VP8 e áudio Opus/Vorbis.
        if let Some(v) = pick(&|f| match vcodec_family(f) {
            "vp9" => Some(3),
            "av1" => Some(2),
            "vp8" => Some(1),
            _ => None,
        }) {
            if v.acodec.as_deref().map(|_| v.ext == "webm").unwrap_or(audio_webm.is_some()) {
                push(Container::Webm, v, audio_webm);
            }
        }
        // MKV: aceita qualquer codec; usa o fluxo de maior taxa de bits.
        if let Some(v) = pick(&|f| Some(if f.vcodec.as_deref() == Some("unknown") { 0 } else { 1 })) {
            push(Container::Mkv, v, audio_any);
        }

        if !containers.is_empty() {
            qualities.push(QualityOption {
                height: *height,
                label: if *height == 0 { "Original".into() } else { quality_label(*height, fps) },
                fps,
                containers,
            });
        }
    }

    let audio = audio_any.map(|a| AudioOption {
        format_id: a.id.clone(),
        acodec: a.acodec.clone().unwrap_or_default(),
        abr: a.abr,
        size_approx: a.size,
    });

    if qualities.is_empty() && audio.is_none() {
        return Err(AppError::new(ErrorCode::NoFormats, "A fonte não oferece nenhum arquivo para download."));
    }

    Ok(MediaInfo {
        kind: MediaKind::Video,
        id,
        url,
        title,
        uploader,
        duration,
        thumbnail,
        source,
        source_label,
        qualities,
        audio,
        entries: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nominal_heights() {
        assert_eq!(nominal_height(Some(1920), Some(1080)), Some(1080));
        assert_eq!(nominal_height(Some(1920), Some(800)), Some(1080));
        assert_eq!(nominal_height(Some(1080), Some(1920)), Some(1080));
        assert_eq!(nominal_height(Some(3840), Some(2160)), Some(2160));
        assert_eq!(nominal_height(Some(640), Some(360)), Some(360));
        assert_eq!(nominal_height(None, Some(720)), Some(720));
        assert_eq!(nominal_height(None, None), None);
        assert_eq!(quality_label(2160, Some(60)), "4K 60fps");
        assert_eq!(quality_label(1080, Some(60)), "1080p60");
        assert_eq!(quality_label(720, Some(30)), "720p");
    }

    fn sample() -> Value {
        json!({
            "id": "abc", "title": "Filme", "extractor_key": "Youtube", "duration": 100.0,
            "webpage_url": "https://www.youtube.com/watch?v=abc", "uploader": "Canal",
            "formats": [
                {"format_id":"sb0","ext":"mhtml","protocol":"mhtml","vcodec":"none","acodec":"none"},
                {"format_id":"140","ext":"m4a","protocol":"https","vcodec":"none","acodec":"mp4a.40.2","abr":129.0,"filesize":1000},
                {"format_id":"140-drc","ext":"m4a","protocol":"https","vcodec":"none","acodec":"mp4a.40.2","abr":130.0,"format_note":"medium, DRC"},
                {"format_id":"258","ext":"m4a","protocol":"https","vcodec":"none","acodec":"mp4a.40.2","abr":384.0,"audio_channels":6,"filesize":3000},
                {"format_id":"773","ext":"m4a","protocol":"https","resolution":"audio only","filesize":900},
                {"format_id":"251","ext":"webm","protocol":"https","vcodec":"none","acodec":"opus","abr":140.0,"filesize":1100},
                {"format_id":"18","ext":"mp4","protocol":"https","vcodec":"avc1.42001E","acodec":"mp4a.40.2","width":640,"height":360,"fps":30,"filesize":5000},
                {"format_id":"136","ext":"mp4","protocol":"https","vcodec":"avc1.4D401F","acodec":"none","width":1280,"height":720,"fps":30,"filesize":8000},
                {"format_id":"247","ext":"webm","protocol":"https","vcodec":"vp9","acodec":"none","width":1280,"height":720,"fps":30,"filesize":7000},
                {"format_id":"401","ext":"mp4","protocol":"https","vcodec":"av01.0.13M.08","acodec":"none","width":3840,"height":2160,"fps":60,"filesize":90000},
                {"format_id":"315","ext":"webm","protocol":"https","vcodec":"vp9","acodec":"none","width":3840,"height":2160,"fps":60,"filesize":120000},
                {"format_id":"628","ext":"mp4","protocol":"m3u8_native","vcodec":"vp09.00.51.08","acodec":"none","width":3840,"height":2160,"fps":60,"tbr":28000.0}
            ]
        })
    }

    #[test]
    fn builds_only_available_qualities() {
        let info = parse_info(&sample(), "x").unwrap();
        let labels: Vec<_> = info.qualities.iter().map(|q| q.label.as_str()).collect();
        assert_eq!(labels, vec!["4K 60fps", "720p", "360p"]);
        assert_eq!(info.source_label, "YouTube");

        let q4k = &info.qualities[0];
        let mp4 = q4k.containers.iter().find(|c| c.container == Container::Mp4).unwrap();
        assert_eq!(mp4.video_format_id, "401"); // AV1 em MP4 (sem H.264 em 4K)
        assert_eq!(mp4.audio_format_id.as_deref(), Some("140")); // AAC, não a faixa DRC
        let webm = q4k.containers.iter().find(|c| c.container == Container::Webm).unwrap();
        assert_eq!(webm.video_format_id, "315"); // https preferido ao m3u8
        assert_eq!(webm.audio_format_id.as_deref(), Some("251"));
        assert_eq!(webm.size_approx, Some(121100));

        let q720 = &info.qualities[1];
        let mp4 = q720.containers.iter().find(|c| c.container == Container::Mp4).unwrap();
        assert_eq!(mp4.video_format_id, "136");
        assert_eq!(mp4.vcodec, "H.264");

        // 360p: fluxo progressivo já contém áudio; não deve ser oferecido WebM.
        let q360 = &info.qualities[2];
        let mp4 = &q360.containers[0];
        assert!(mp4.video_has_audio && mp4.audio_format_id.is_none());
        assert!(q360.containers.iter().all(|c| c.container != Container::Webm));

        assert_eq!(info.audio.unwrap().format_id, "251");
    }

    #[test]
    fn rejects_live_and_empty() {
        let mut v = sample();
        v["is_live"] = json!(true);
        assert_eq!(parse_info(&v, "x").unwrap_err().code, ErrorCode::Live);
        let empty = json!({"id":"x","title":"t","formats":[{"format_id":"a","ext":"mp4","has_drm":true}]});
        assert_eq!(parse_info(&empty, "x").unwrap_err().code, ErrorCode::Drm);
    }

    #[test]
    fn direct_file_without_codecs() {
        let v = json!({"id":"f","title":"clip","extractor_key":"Generic","ext":"mp4","url":"https://h/x.mp4",
                       "formats":[{"format_id":"mp4","ext":"mp4","url":"https://h/x.mp4","protocol":"https"}]});
        let info = parse_info(&v, "x").unwrap();
        assert_eq!(info.qualities.len(), 1);
        assert_eq!(info.qualities[0].label, "Original");
        assert!(info.qualities[0].containers[0].video_has_audio);
    }

    #[test]
    fn playlist_entries() {
        let v = json!({"_type":"playlist","id":"PL1","title":"Lista","extractor_key":"YoutubeTab",
            "entries":[{"url":"https://www.youtube.com/watch?v=a","title":"A"},{"url":"javascript:x"}]});
        let info = parse_info(&v, "x").unwrap();
        assert_eq!(info.kind, MediaKind::Playlist);
        assert_eq!(info.entries.len(), 1);
    }
}
