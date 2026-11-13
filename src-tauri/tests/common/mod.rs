//! Infraestrutura dos testes de integração: servidor HTTP local com suporte a
//! Range e limitação de velocidade, mídia de teste gerada pelo FFmpeg empacotado
//! e um coletor de eventos do gerenciador.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once, OnceLock, RwLock};
use std::time::{Duration, Instant};

use vibax_lib::downloads::job::{Job, JobStatus};
use vibax_lib::downloads::manager::{EventSink, Manager};
use vibax_lib::storage::db::Db;
use vibax_lib::storage::settings::Settings;
use vibax_lib::tools;

pub fn tools_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools")
}

/// Remove o PATH global: prova que nada depende de executáveis instalados na máquina.
pub fn isolate_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let sys = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        std::env::set_var("PATH", format!("{sys}\\System32"));
    });
}

/// Gera (uma vez) os arquivos de mídia de teste com o FFmpeg empacotado.
pub fn media() -> &'static (Vec<u8>, Vec<u8>) {
    static MEDIA: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    MEDIA.get_or_init(|| {
        let dir = std::env::temp_dir().join("vibax-test-media");
        std::fs::create_dir_all(&dir).unwrap();
        let ffmpeg = tools_dir().join("ffmpeg.exe");
        let gen = |name: &str, secs: &str, bitrate: &str| -> Vec<u8> {
            let out = dir.join(name);
            if !out.is_file() {
                let st = std::process::Command::new(&ffmpeg)
                    .args(["-v", "error", "-y", "-f", "lavfi", "-i", &format!("testsrc2=size=640x360:rate=30:duration={secs}")])
                    .args(["-f", "lavfi", "-i", &format!("sine=frequency=440:duration={secs}")])
                    .args(["-c:v", "libx264", "-preset", "ultrafast", "-b:v", bitrate, "-c:a", "aac", "-shortest"])
                    .args(["-movflags", "+faststart"])
                    .arg(&out)
                    .status()
                    .expect("ffmpeg");
                assert!(st.success());
            }
            std::fs::read(&out).unwrap()
        };
        (gen("clip.mp4", "4", "400k"), gen("slow.mp4", "20", "3000k"))
    })
}

pub struct TestServer {
    pub base: String,
    pub ranges: Arc<Mutex<Vec<String>>>,
}

const SLOW_BPS: usize = 1_200_000;

pub fn server() -> &'static TestServer {
    static SERVER: OnceLock<TestServer> = OnceLock::new();
    SERVER.get_or_init(|| {
        let (clip, slow) = media().clone();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let ranges: Arc<Mutex<Vec<String>>> = Default::default();
        let r2 = ranges.clone();
        let clip = Arc::new(clip);
        let slow = Arc::new(slow);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (clip, slow, ranges) = (clip.clone(), slow.clone(), r2.clone());
                std::thread::spawn(move || {
                    let _ = handle(stream, &clip, &slow, &ranges);
                });
            }
        });
        TestServer { base: format!("http://127.0.0.1:{port}"), ranges }
    })
}

fn handle(mut s: TcpStream, clip: &[u8], slow: &[u8], ranges: &Mutex<Vec<String>>) -> std::io::Result<()> {
    let mut reader = BufReader::new(s.try_clone()?);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut range: Option<usize> = None;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h == "\r\n" {
            break;
        }
        let lower = h.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("range: bytes=") {
            let v = v.trim();
            ranges.lock().unwrap().push(v.to_string());
            range = v.split('-').next().and_then(|n| n.parse().ok());
        }
    }
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("").split('?').next().unwrap_or("");
    let (body, ctype, throttle): (&[u8], &str, bool) = match path {
        "/clip.mp4" => (clip, "video/mp4", false),
        "/slow.mp4" => (slow, "video/mp4", true),
        "/page.html" => (b"<html><head><title>Sem video</title></head><body>Nada aqui.</body></html>", "text/html", false),
        _ => {
            s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
            return Ok(());
        }
    };
    let start = range.unwrap_or(0).min(body.len());
    let slice = &body[start..];
    let status = if range.is_some() { "206 Partial Content" } else { "200 OK" };
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n",
        slice.len()
    );
    if range.is_some() {
        head.push_str(&format!("Content-Range: bytes {start}-{}/{}\r\n", body.len() - 1, body.len()));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())?;
    if method == "HEAD" {
        return Ok(());
    }
    if throttle {
        let chunk = SLOW_BPS / 20;
        for c in slice.chunks(chunk) {
            s.write_all(c)?;
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        s.write_all(slice)?;
    }
    let mut sink = [0u8; 1];
    let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
    let _ = s.read(&mut sink);
    Ok(())
}

#[derive(Default)]
pub struct Recorder {
    pub events: Mutex<Vec<(String, JobStatus, Option<f64>)>>,
    pub removed: Mutex<Vec<String>>,
    pub finished: Mutex<Vec<String>>,
}

impl EventSink for Recorder {
    fn job_updated(&self, job: &Job) {
        self.events.lock().unwrap().push((job.id.clone(), job.status, job.progress));
    }
    fn job_removed(&self, id: &str) {
        self.removed.lock().unwrap().push(id.to_string());
    }
    fn job_finished(&self, job: &Job) {
        self.finished.lock().unwrap().push(job.id.clone());
    }
}

impl Recorder {
    pub fn statuses(&self, id: &str) -> Vec<JobStatus> {
        let mut v: Vec<JobStatus> = Vec::new();
        for (j, s, _) in self.events.lock().unwrap().iter() {
            if j == id && v.last() != Some(s) {
                v.push(*s);
            }
        }
        v
    }
}

pub struct Ctx {
    pub manager: Arc<Manager>,
    pub rec: Arc<Recorder>,
    pub dir: tempfile::TempDir,
    pub db_path: PathBuf,
    pub settings: Arc<RwLock<Settings>>,
}

pub fn ctx() -> Ctx {
    isolate_env();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("vibax.db");
    let mut s = Settings::default();
    s.download_dir = dir.path().join("downloads").display().to_string();
    s.max_concurrent = 2;
    s.embed_metadata = true;
    let settings = Arc::new(RwLock::new(s));
    let rec = Arc::new(Recorder::default());
    let manager = manager_for(&db_path, rec.clone(), settings.clone(), dir.path());
    Ctx { manager, rec, dir, db_path, settings }
}

pub fn manager_for(db_path: &Path, rec: Arc<Recorder>, settings: Arc<RwLock<Settings>>, root: &Path) -> Arc<Manager> {
    let paths = tools::resolve(None, &root.join("localdata"), root.join("cache")).expect("componentes empacotados");
    let db = Arc::new(Db::open(db_path).unwrap());
    Manager::new(db, rec, settings, Arc::new(paths), None)
}

pub async fn wait_for(m: &Manager, id: &str, secs: u64, pred: impl Fn(&Job) -> bool) -> Job {
    let start = Instant::now();
    loop {
        if let Some(j) = m.get(id) {
            if pred(&j) {
                return j;
            }
            if start.elapsed() > Duration::from_secs(secs) {
                panic!("tempo esgotado aguardando a tarefa; estado atual: {:?} / {:?} / {:?}", j.status, j.stage, j.error);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Usa o ffprobe empacotado para listar os tipos de fluxo de um arquivo.
pub fn stream_types(file: &Path) -> Vec<String> {
    let out = std::process::Command::new(tools_dir().join("ffprobe.exe"))
        .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

pub fn duration_of(file: &Path) -> f64 {
    let out = std::process::Command::new(tools_dir().join("ffprobe.exe"))
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0.0)
}
