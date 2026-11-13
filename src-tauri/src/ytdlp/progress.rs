//! Interpretação das linhas emitidas pelo yt-dlp com os modelos definidos em `spec`.

use super::spec::{TAG_DL, TAG_FILE, TAG_FMT, TAG_PP};

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Format { format_id: String, ext: String, size: Option<u64> },
    Download { finished: bool, downloaded: Option<u64>, total: Option<u64>, speed: Option<f64>, eta: Option<u64> },
    PostProcess { started: bool, name: String },
    File(String),
    Error(String),
    Other,
}

fn num(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s == "NA" || s == "None" {
        return None;
    }
    s.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0)
}

pub fn parse_line(raw: &str) -> Line {
    let line = raw.trim_end_matches(['\r', '\n']);
    if let Some(rest) = line.strip_prefix(TAG_DL) {
        let p: Vec<&str> = rest.split('|').collect();
        if p.len() >= 6 {
            let total = num(p[2]).or_else(|| num(p[3]));
            return Line::Download {
                finished: p[0] == "finished",
                downloaded: num(p[1]).map(|x| x as u64),
                total: total.map(|x| x as u64),
                speed: num(p[4]),
                eta: num(p[5]).map(|x| x as u64),
            };
        }
    } else if let Some(rest) = line.strip_prefix(TAG_PP) {
        let (status, name) = rest.split_once('|').unwrap_or((rest, ""));
        return Line::PostProcess { started: status == "started", name: name.to_string() };
    } else if let Some(rest) = line.strip_prefix(TAG_FMT) {
        let p: Vec<&str> = rest.splitn(3, '|').collect();
        return Line::Format {
            format_id: p.first().unwrap_or(&"").to_string(),
            ext: p.get(1).unwrap_or(&"").to_string(),
            size: p.get(2).and_then(|s| num(s)).map(|x| x as u64),
        };
    } else if let Some(rest) = line.strip_prefix(TAG_FILE) {
        return Line::File(rest.trim().to_string());
    } else if let Some(rest) = line.strip_prefix("ERROR:") {
        return Line::Error(rest.trim().to_string());
    }
    Line::Other
}

/// Nome amigável da etapa de pós-processamento.
pub fn stage_label(postprocessor: &str) -> &'static str {
    match postprocessor {
        "Merger" => "Combinando vídeo e áudio",
        "ExtractAudio" => "Extraindo áudio",
        "VideoRemuxer" | "VideoConvertor" => "Convertendo o formato",
        "EmbedThumbnail" => "Incorporando a capa",
        "Metadata" | "FFmpegMetadata" => "Gravando metadados",
        "FixupM3u8" | "FixupM4a" | "FixupDuplicateMoov" | "FixupTimestamp" | "FixupDuration" | "FixupStretched" => {
            "Corrigindo o arquivo"
        }
        "MoveFiles" => "Finalizando",
        _ => "Processando",
    }
}

/// Converte o progresso de várias partes (vídeo + áudio) em um percentual único.
#[derive(Debug, Default, Clone)]
pub struct Tracker {
    pub parts: usize,
    pub parts_done: usize,
    pub size: Option<u64>,
    pub ext: Option<String>,
    current_total: Option<u64>,
    bytes_done_before: u64,
    pub downloaded: u64,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
}

impl Tracker {
    pub fn on_format(&mut self, format_id: &str, ext: &str, size: Option<u64>) {
        self.parts = format_id.split('+').count().max(1);
        self.parts_done = 0;
        self.bytes_done_before = 0;
        self.size = size;
        self.ext = Some(ext.to_string()).filter(|e| !e.is_empty());
    }

    pub fn on_download(&mut self, finished: bool, downloaded: Option<u64>, total: Option<u64>, speed: Option<f64>, eta: Option<u64>) {
        if self.parts == 0 {
            self.parts = 1;
        }
        self.current_total = total.or(self.current_total);
        let cur = downloaded.unwrap_or(0);
        self.downloaded = self.bytes_done_before + cur;
        self.speed = speed;
        self.eta = eta;
        if finished {
            self.bytes_done_before += cur.max(total.unwrap_or(0));
            self.downloaded = self.bytes_done_before;
            self.parts_done = (self.parts_done + 1).min(self.parts);
            self.current_total = None;
            self.speed = None;
            self.eta = None;
        }
    }

    /// Fração concluída (0..=1) se puder ser estimada.
    pub fn fraction(&self) -> Option<f64> {
        if self.parts == 0 {
            return None;
        }
        if self.parts_done >= self.parts {
            return Some(1.0);
        }
        // Com o tamanho total conhecido, usa bytes (mais preciso que contar partes).
        if let Some(size) = self.size.filter(|s| *s > 0) {
            return Some((self.downloaded as f64 / size as f64).clamp(0.0, 0.999));
        }
        let cur = match self.current_total {
            Some(t) if t > 0 => (self.downloaded - self.bytes_done_before) as f64 / t as f64,
            _ => return if self.parts_done > 0 { Some(self.parts_done as f64 / self.parts as f64) } else { None },
        };
        Some(((self.parts_done as f64 + cur.clamp(0.0, 1.0)) / self.parts as f64).clamp(0.0, 0.999))
    }

    /// Tamanho total esperado (informado pela fonte ou pela parte atual).
    pub fn total(&self) -> Option<u64> {
        self.size.or_else(|| (self.parts <= 1).then_some(self.current_total).flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tagged_lines() {
        assert_eq!(
            parse_line("[vibax-dl]downloading|2096128|10271496|NA|15815288.68|3\r\n"),
            Line::Download { finished: false, downloaded: Some(2096128), total: Some(10271496), speed: Some(15815288.68), eta: Some(3) }
        );
        assert_eq!(
            parse_line("[vibax-dl]downloading|100|NA|5000.5|NA|NA"),
            Line::Download { finished: false, downloaded: Some(100), total: Some(5000), speed: None, eta: None }
        );
        assert_eq!(
            parse_line("[vibax-fmt]134+140|mp4|28565606"),
            Line::Format { format_id: "134+140".into(), ext: "mp4".into(), size: Some(28565606) }
        );
        assert_eq!(parse_line("[vibax-pp]started|Merger"), Line::PostProcess { started: true, name: "Merger".into() });
        assert_eq!(parse_line("[vibax-file]C:\\a\\Título | x.mp4"), Line::File("C:\\a\\Título | x.mp4".into()));
        assert_eq!(parse_line("ERROR: [youtube] x: Video unavailable"), Line::Error("[youtube] x: Video unavailable".into()));
        assert_eq!(parse_line("[download] Destination: x"), Line::Other);
    }

    #[test]
    fn tracks_two_parts() {
        let mut t = Tracker::default();
        t.on_format("137+140", "mp4", Some(1000));
        t.on_download(false, Some(400), Some(800), Some(10.0), Some(5));
        assert!((t.fraction().unwrap() - 0.4).abs() < 1e-9);
        t.on_download(true, Some(800), Some(800), None, None);
        t.on_download(false, Some(100), Some(200), Some(1.0), Some(1));
        assert!((t.fraction().unwrap() - 0.9).abs() < 1e-9);
        t.on_download(true, Some(200), Some(200), None, None);
        assert_eq!(t.fraction(), Some(1.0));
    }

    #[test]
    fn tracks_parts_without_size() {
        let mut t = Tracker::default();
        t.on_format("137+140", "mp4", None);
        t.on_download(false, Some(50), Some(100), None, None);
        assert!((t.fraction().unwrap() - 0.25).abs() < 1e-9);
        t.on_download(true, Some(100), Some(100), None, None);
        assert!((t.fraction().unwrap() - 0.5).abs() < 1e-9);
    }
}
