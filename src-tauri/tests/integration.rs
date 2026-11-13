//! Testes de integração do fluxo completo, usando o yt-dlp/FFmpeg/Deno
//! empacotados (o PATH global é removido em `isolate_env`).
//!
//! Fontes usadas (todas com download autorizado):
//! - vídeos gerados pelo próprio teste com o FFmpeg, servidos localmente;
//! - "Big Buck Bunny" (Blender Foundation, licença Creative Commons BY 3.0),
//!   publicado no canal oficial da Blender no YouTube.
//!
//! Testes de rede externa podem ser pulados com VIBAX_SKIP_NETWORK=1.

mod common;

use std::path::Path;
use std::sync::Arc;

use common::*;
use vibax_lib::downloads::job::JobStatus;
use vibax_lib::error::ErrorCode;
use vibax_lib::ytdlp::info::{AudioFormat, Container, MediaKind};
use vibax_lib::ytdlp::spec::{DownloadChoice, Preference};

const BBB: &str = "https://www.youtube.com/watch?v=aqz-KE-bpKQ";

fn skip_network() -> bool {
    std::env::var("VIBAX_SKIP_NETWORK").is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn analyze_local_file() {
    let c = ctx();
    let url = format!("{}/clip.mp4", server().base);
    let info = c.manager.analyze(&url).await.expect("análise");
    assert_eq!(info.kind, MediaKind::Video);
    assert!(!info.qualities.is_empty());
    assert!(info.qualities[0].containers.iter().any(|o| o.container == Container::Mp4));
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_and_unsupported_links() {
    let c = ctx();
    let e = c.manager.analyze("file:///C:/Windows/win.ini").await.unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidUrl);
    let e = c.manager.analyze("--exec calc").await.unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidUrl);

    let e = c.manager.analyze(&format!("{}/page.html", server().base)).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Unsupported, "{e:?}");

    let e = c.manager.analyze(&format!("{}/nada.mp4", server().base)).await.unwrap_err();
    assert!(matches!(e.code, ErrorCode::Unavailable | ErrorCode::Network), "{e:?}");

    // Porta fechada: falha de rede.
    let e = c.manager.analyze("http://127.0.0.1:9/video.mp4").await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Network, "{e:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn batch_download_completes_and_persists_history() {
    let c = ctx();
    let url = format!("{}/clip.mp4", server().base);
    let jobs = c
        .manager
        .enqueue_many(&[url.clone()], Some(Preference::Video { max_height: 1080, container: Container::Mp4 }), None)
        .unwrap();
    let id = jobs[0].id.clone();
    let job = wait_for(&c.manager, &id, 90, |j| j.status.is_finished()).await;
    assert_eq!(job.status, JobStatus::Completed, "{:?}", job.error);

    let path = Path::new(job.file_path.as_deref().unwrap()).to_path_buf();
    assert!(path.is_file());
    assert_eq!(path.extension().unwrap(), "mp4");
    assert_eq!(job.file_size, Some(std::fs::metadata(&path).unwrap().len()));
    assert_eq!(stream_types(&path), vec!["video", "audio"]);
    // Pasta temporária removida após concluir.
    assert!(!Path::new(&job.dest_dir).join(".vibax-tmp").exists());

    // Estados percorridos: aguardando → analisando → baixando → concluído.
    let st = c.rec.statuses(&id);
    assert_eq!(st.first(), Some(&JobStatus::Queued));
    assert!(st.contains(&JobStatus::Analyzing));
    assert!(st.contains(&JobStatus::Downloading));
    assert_eq!(st.last(), Some(&JobStatus::Completed));
    assert!(c.rec.finished.lock().unwrap().contains(&id));

    // Histórico persistido e recuperado por uma nova instância (como ao reabrir o app).
    let hist = c.manager.history(None).unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0].file_exists, Some(true));
    let m2 = manager_for(&c.db_path, Arc::new(Recorder::default()), c.settings.clone(), c.dir.path());
    m2.restore().unwrap();
    assert_eq!(m2.history(Some("clip")).unwrap().len(), 1);
    assert_eq!(m2.history(Some("inexistente")).unwrap().len(), 0);
    assert_eq!(m2.queue().len(), 1);

    // Remover da fila mantém o histórico; remover do histórico apaga o registro.
    m2.remove(&id).unwrap();
    assert!(m2.queue().is_empty());
    assert_eq!(m2.history(None).unwrap().len(), 1);
    m2.remove_history(&id).unwrap();
    assert!(m2.history(None).unwrap().is_empty());

    // Arquivo movido/excluído é detectado no histórico.
    let id2 = c.manager.redownload(&id).unwrap().id;
    let j2 = wait_for(&c.manager, &id2, 90, |j| j.status.is_finished()).await;
    assert_eq!(j2.status, JobStatus::Completed);
    std::fs::remove_file(j2.file_path.as_deref().unwrap()).unwrap();
    let h = c.manager.history(None).unwrap();
    assert_eq!(h.iter().find(|j| j.id == id2).unwrap().file_exists, Some(false));
    assert_eq!(c.manager.file_of(&id2).unwrap_err().code, ErrorCode::NotFound);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_removes_partial_files() {
    let c = ctx();
    let url = format!("{}/slow.mp4", server().base);
    let id = c.manager.enqueue_many(&[url], None, None).unwrap()[0].id.clone();
    let j = wait_for(&c.manager, &id, 60, |j| j.status == JobStatus::Downloading && j.downloaded_bytes > 300_000).await;
    let temp = Path::new(&j.dest_dir).join(".vibax-tmp").join(&id);
    assert!(temp.is_dir(), "arquivos parciais devem existir durante o download");

    c.manager.cancel(&id).unwrap();
    let j = wait_for(&c.manager, &id, 20, |j| j.status.is_finished()).await;
    assert_eq!(j.status, JobStatus::Cancelled);
    assert!(!temp.exists(), "cancelar deve remover os parciais");
    let files: Vec<_> = std::fs::read_dir(&j.dest_dir).unwrap().flatten().collect();
    assert!(files.is_empty(), "nenhum arquivo final deve ficar: {files:?}");

    // Tarefa cancelada pode ser repetida e concluída.
    c.manager.retry(&id).unwrap();
    let j = wait_for(&c.manager, &id, 90, |j| j.status.is_finished()).await;
    assert_eq!(j.status, JobStatus::Completed, "{:?}", j.error);
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_and_resume_continue_from_partial() {
    let c = ctx();
    let url = format!("{}/slow.mp4?pause", server().base);
    let id = c.manager.enqueue_many(&[url], None, None).unwrap()[0].id.clone();
    wait_for(&c.manager, &id, 60, |j| j.status == JobStatus::Downloading && j.downloaded_bytes > 1_500_000).await;

    c.manager.pause(&id).unwrap();
    let j = wait_for(&c.manager, &id, 20, |j| j.status == JobStatus::Paused).await;
    let temp = Path::new(&j.dest_dir).join(".vibax-tmp").join(&id);
    let partial: u64 = std::fs::read_dir(&temp).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum();
    assert!(partial > 1_000_000, "o arquivo parcial deve ser mantido ao pausar ({partial} bytes)");

    let before = server().ranges.lock().unwrap().len();
    c.manager.resume(&id).unwrap();
    let j = wait_for(&c.manager, &id, 90, |j| j.status.is_finished()).await;
    assert_eq!(j.status, JobStatus::Completed, "{:?}", j.error);

    // A retomada pediu ao servidor apenas o restante do arquivo (Range a partir do parcial).
    let ranges = server().ranges.lock().unwrap()[before..].to_vec();
    let resumed_from: Vec<u64> = ranges.iter().filter_map(|r| r.split('-').next()?.parse().ok()).collect();
    assert!(resumed_from.iter().any(|&b| b >= 1_000_000), "esperava Range de retomada, obtido {ranges:?}");

    // O arquivo final é íntegro: mesmos fluxos e mesma duração do original
    // (o tamanho difere em poucos bytes porque o Vibax grava metadados).
    let out = Path::new(j.file_path.as_deref().unwrap()).to_path_buf();
    assert_eq!(stream_types(&out), vec!["video", "audio"]);
    let d = duration_of(&out);
    assert!((d - 20.0).abs() < 0.5, "duração inesperada: {d}");
}

#[tokio::test(flavor = "multi_thread")]
async fn queue_respects_concurrency_limit() {
    let c = ctx();
    c.settings.write().unwrap().max_concurrent = 1;
    let base = server().base.clone();
    let urls: Vec<String> = (0..3).map(|i| format!("{base}/clip.mp4?n={i}")).collect();
    let ids: Vec<String> = c.manager.enqueue_many(&urls, None, None).unwrap().into_iter().map(|j| j.id).collect();
    // Enquanto o primeiro roda, os demais aguardam.
    let mut max_active = 0;
    let start = std::time::Instant::now();
    loop {
        let q = c.manager.queue();
        max_active = max_active.max(q.iter().filter(|j| j.status.is_active()).count());
        if q.iter().all(|j| j.status.is_finished()) || start.elapsed().as_secs() > 180 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(max_active, 1);
    for id in &ids {
        assert_eq!(c.manager.get(id).unwrap().status, JobStatus::Completed);
    }
    // "Limpar concluídos" esvazia a fila e mantém o histórico.
    assert_eq!(c.manager.clear_finished().unwrap(), 3);
    assert!(c.manager.queue().is_empty());
    assert_eq!(c.manager.history(None).unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn permission_error_on_protected_folder() {
    let c = ctx();
    let url = format!("{}/clip.mp4", server().base);
    let sys = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let dest = format!("{sys}\\System32\\vibax-teste-sem-permissao");
    let id = c.manager.enqueue_many(&[url], None, Some(dest)).unwrap()[0].id.clone();
    let j = wait_for(&c.manager, &id, 60, |j| j.status.is_finished()).await;
    assert_eq!(j.status, JobStatus::Error);
    assert_eq!(j.error.unwrap().code, ErrorCode::Permission);
}

#[tokio::test(flavor = "multi_thread")]
async fn interrupted_jobs_are_restored_as_paused() {
    let c = ctx();
    let url = format!("{}/slow.mp4?restore", server().base);
    let id = c.manager.enqueue_many(&[url], None, None).unwrap()[0].id.clone();
    wait_for(&c.manager, &id, 60, |j| j.status == JobStatus::Downloading && j.downloaded_bytes > 200_000).await;
    // Simula o fechamento do app no meio do download.
    c.manager.shutdown();
    let m2 = manager_for(&c.db_path, Arc::new(Recorder::default()), c.settings.clone(), c.dir.path());
    m2.restore().unwrap();
    let j = m2.get(&id).unwrap();
    assert_eq!(j.status, JobStatus::Paused);
    assert!(j.stage.unwrap().contains("Interrompido"));
    wait_for(&c.manager, &id, 20, |j| j.status == JobStatus::Paused).await;
    m2.resume(&id).unwrap();
    let j = wait_for(&m2, &id, 90, |j| j.status.is_finished()).await;
    assert_eq!(j.status, JobStatus::Completed, "{:?}", j.error);
}

// ----------------------------------------------------------- rede externa

#[tokio::test(flavor = "multi_thread")]
async fn youtube_cc_video_analysis_and_downloads() {
    if skip_network() {
        return;
    }
    let c = ctx();
    let info = c.manager.analyze(BBB).await.expect("análise do YouTube");
    assert_eq!(info.source_label, "YouTube");
    assert!(info.title.contains("Big Buck Bunny"));
    assert!(info.duration.unwrap() > 600.0);
    assert!(info.thumbnail.is_some());
    let heights: Vec<u32> = info.qualities.iter().map(|q| q.height).collect();
    for h in [2160, 1440, 1080, 720, 480, 360] {
        assert!(heights.contains(&h), "qualidade {h} ausente: {heights:?}");
    }
    assert!(!heights.contains(&4320), "não pode oferecer 8K inexistente");

    // 360p MP4: vídeo e áudio separados, combinados pelo FFmpeg.
    let q = info.qualities.iter().find(|q| q.height == 360).unwrap();
    let opt = q.containers.iter().find(|o| o.container == Container::Mp4).unwrap();
    assert!(!opt.video_has_audio, "deve preferir vídeo adaptativo + áudio separado ao itag 18");
    let choice = DownloadChoice::Video {
        height: 360,
        label: q.label.clone(),
        container: Container::Mp4,
        video_format_id: Some(opt.video_format_id.clone()),
        audio_format_id: opt.audio_format_id.clone(),
        video_has_audio: opt.video_has_audio,
    };
    let v = c.manager.enqueue(BBB, Some(choice), None, None).unwrap();
    assert_eq!(v.quality_label.as_deref(), Some("360p"));

    // Áudio MP3 (extração + conversão).
    let a = c
        .manager
        .enqueue(
            BBB,
            Some(DownloadChoice::Audio {
                format: AudioFormat::Mp3,
                bitrate: Some(128),
                audio_format_id: info.audio.as_ref().map(|a| a.format_id.clone()),
            }),
            None,
            None,
        )
        .unwrap();

    let v = wait_for(&c.manager, &v.id, 300, |j| j.status.is_finished()).await;
    assert_eq!(v.status, JobStatus::Completed, "{:?}", v.error);
    let vp = Path::new(v.file_path.as_deref().unwrap());
    assert!(vp.file_name().unwrap().to_string_lossy().contains("(360p)"));
    assert_eq!(vp.extension().unwrap(), "mp4");
    let types = stream_types(vp);
    assert!(types.contains(&"video".to_string()) && types.contains(&"audio".to_string()), "{types:?}");
    assert!(c.rec.statuses(&v.id).contains(&JobStatus::Processing), "esperava etapa de mesclagem");

    let a = wait_for(&c.manager, &a.id, 300, |j| j.status.is_finished()).await;
    assert_eq!(a.status, JobStatus::Completed, "{:?}", a.error);
    let ap = Path::new(a.file_path.as_deref().unwrap());
    assert_eq!(ap.extension().unwrap(), "mp3");
    assert!(stream_types(ap).contains(&"audio".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn youtube_unavailable_video_reports_clear_error() {
    if skip_network() {
        return;
    }
    let c = ctx();
    let e = c.manager.analyze("https://www.youtube.com/watch?v=xxxxxxxxxx0").await.unwrap_err();
    assert!(matches!(e.code, ErrorCode::Unavailable | ErrorCode::AuthRequired), "{e:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn ytdlp_update_downloads_verifies_and_activates() {
    if skip_network() {
        return;
    }
    isolate_env();
    let dir = tempfile::tempdir().unwrap();
    // Simula uma versão antiga instalada para forçar o download completo.
    let r = vibax_lib::updater::update(dir.path(), Some("2000.01.01".into())).await.expect("atualização");
    assert!(r.updated);
    let active = vibax_lib::tools::updated_ytdlp_dir(dir.path()).expect("versão ativada");
    assert!(active.join("yt-dlp.exe").is_file());
    let paths = vibax_lib::tools::resolve(None, dir.path(), dir.path().join("cache")).unwrap();
    assert!(paths.ytdlp_updated, "o app deve passar a usar a versão atualizada");
    assert_eq!(paths.tools_info_version().await.as_deref(), Some(r.latest.as_str()));

    vibax_lib::updater::reset(dir.path()).unwrap();
    let paths = vibax_lib::tools::resolve(None, dir.path(), dir.path().join("cache")).unwrap();
    assert!(!paths.ytdlp_updated, "restaurar volta à versão empacotada");
}

#[tokio::test(flavor = "multi_thread")]
async fn youtube_playlist_lists_entries() {
    if skip_network() {
        return;
    }
    let c = ctx();
    // Canal oficial da Blender Studio (vídeos publicados sob Creative Commons).
    let info = c.manager.analyze("https://www.youtube.com/@BlenderStudio/videos").await.expect("análise da lista");
    assert_eq!(info.kind, MediaKind::Playlist);
    assert!(info.entries.len() > 10, "esperava vários vídeos: {}", info.entries.len());
    assert!(info.entries.iter().all(|e| e.url.starts_with("https://")));
    assert!(info.entries.iter().filter(|e| e.title.is_some()).count() > 10);
}
