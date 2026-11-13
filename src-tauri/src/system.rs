//! Recursos nativos do Windows: espaço livre, verificação de escrita e pastas temporárias.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult, ErrorCode};

/// Bytes livres disponíveis para o usuário no volume que contém `path`.
#[cfg(windows)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    // Sobe até um diretório existente (a pasta de destino pode ainda não existir).
    let mut p = path.to_path_buf();
    while !p.exists() {
        p = p.parent()?.to_path_buf();
    }
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut avail: u64 = 0;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, std::ptr::null_mut(), std::ptr::null_mut()) };
    (ok != 0).then_some(avail)
}

#[cfg(not(windows))]
pub fn free_space(_path: &Path) -> Option<u64> {
    None
}

pub fn format_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", U[i]).replace('.', ",")
    }
}

/// Garante que a pasta existe e que o Vibax consegue gravar nela.
pub fn ensure_writable_dir(dir: &Path) -> AppResult<()> {
    if !dir.is_absolute() {
        return Err(AppError::new(ErrorCode::Permission, "A pasta de destino precisa ser um caminho completo."));
    }
    std::fs::create_dir_all(dir).map_err(|e| {
        AppError::new(ErrorCode::Permission, "Não foi possível criar a pasta de destino.").with_detail(e.to_string())
    })?;
    let probe = dir.join(format!(".vibax-write-test-{}", uuid::Uuid::new_v4().simple()));
    match std::fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::StorageFull => {
            Err(AppError::new(ErrorCode::DiskFull, "O disco de destino está cheio.").with_detail(e.to_string()))
        }
        Err(e) => Err(AppError::new(
            ErrorCode::Permission,
            "Sem permissão para gravar na pasta de destino. Escolha outra pasta.",
        )
        .with_detail(format!("{}: {e}", dir.display()))),
    }
}

/// Verifica se há espaço para o tamanho estimado. Quando vídeo e áudio são
/// baixados separadamente, a mesclagem precisa temporariamente do dobro.
pub fn check_space(dir: &Path, estimated: Option<u64>, needs_merge: bool) -> AppResult<()> {
    let (Some(size), Some(free)) = (estimated, free_space(dir)) else { return Ok(()) };
    let factor = if needs_merge { 2.0 } else { 1.1 };
    let needed = (size as f64 * factor) as u64 + 32 * 1024 * 1024;
    if free < needed {
        return Err(AppError::new(
            ErrorCode::DiskFull,
            format!(
                "Espaço insuficiente: são necessários cerca de {} e há {} livres no disco de destino.",
                format_bytes(needed),
                format_bytes(free)
            ),
        ));
    }
    Ok(())
}

pub const TEMP_ROOT: &str = ".vibax-tmp";

/// Pasta de trabalho de uma tarefa, no mesmo disco do destino (move instantâneo ao final).
pub fn job_temp_dir(dest: &Path, job_id: &str) -> PathBuf {
    dest.join(TEMP_ROOT).join(job_id)
}

pub fn prepare_temp_dir(dest: &Path, job_id: &str) -> AppResult<PathBuf> {
    let root = dest.join(TEMP_ROOT);
    let dir = root.join(job_id);
    std::fs::create_dir_all(&dir)?;
    set_hidden(&root);
    Ok(dir)
}

pub fn cleanup_temp_dir(dest: &Path, job_id: &str) {
    let dir = job_temp_dir(dest, job_id);
    let _ = std::fs::remove_dir_all(&dir);
    // Remove a pasta raiz se não houver outras tarefas usando-a.
    let _ = std::fs::remove_dir(dest.join(TEMP_ROOT));
}

#[cfg(windows)]
fn set_hidden(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
    }
}

#[cfg(not(windows))]
fn set_hidden(_path: &Path) {}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writable_and_space() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("a").join("b");
        ensure_writable_dir(&sub).unwrap();
        assert!(sub.is_dir());
        assert!(free_space(&sub.join("nao-existe")).unwrap() > 0);
        assert!(check_space(&sub, Some(1024), true).is_ok());
        let err = check_space(&sub, Some(u64::MAX / 4), false).unwrap_err();
        assert_eq!(err.code, ErrorCode::DiskFull);
        assert!(ensure_writable_dir(Path::new("relativo")).is_err());
    }

    #[test]
    fn temp_dirs() {
        let d = tempfile::tempdir().unwrap();
        let t = prepare_temp_dir(d.path(), "job1").unwrap();
        std::fs::write(t.join("x.part"), b"1").unwrap();
        cleanup_temp_dir(d.path(), "job1");
        assert!(!d.path().join(TEMP_ROOT).exists());
    }

    #[test]
    fn bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1,5 KB");
    }
}
