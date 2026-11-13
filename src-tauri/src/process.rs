//! Execução de processos externos (yt-dlp/FFmpeg) com segurança:
//! - argumentos passados como vetor (sem shell, sem interpolação);
//! - sem janela de console;
//! - cada tarefa roda dentro de um Job Object do Windows, de forma que cancelar
//!   encerra a árvore inteira (yt-dlp → ffmpeg/deno) e, se o Vibax fechar
//!   inesperadamente, nenhum processo filho fica órfão.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Monta um `Command` com ambiente controlado.
pub fn command(program: &Path, args: &[impl AsRef<OsStr>]) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Garante saída em UTF-8 (títulos com acentos/emoji) a partir do Python do yt-dlp.
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .env("NO_COLOR", "1")
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Grupo de processos que pode ser encerrado de uma só vez.
pub struct ProcessGroup {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// O HANDLE de um Job Object pode ser usado de qualquer thread.
unsafe impl Send for ProcessGroup {}
unsafe impl Sync for ProcessGroup {}

impl ProcessGroup {
    #[cfg(windows)]
    pub fn new() -> std::io::Result<Self> {
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                let err = std::io::Error::last_os_error();
                windows_sys::Win32::Foundation::CloseHandle(handle);
                return Err(err);
            }
            Ok(Self { handle })
        }
    }

    #[cfg(not(windows))]
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {})
    }

    /// Associa o processo ao grupo (filhos criados depois herdam o grupo).
    #[cfg(windows)]
    pub fn assign(&self, child: &Child) -> std::io::Result<()> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        let raw = child
            .raw_handle()
            .ok_or_else(|| std::io::Error::other("processo já finalizado"))?;
        let ok = unsafe { AssignProcessToJobObject(self.handle, raw as _) };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn assign(&self, _child: &Child) -> std::io::Result<()> {
        Ok(())
    }

    /// Encerra todos os processos do grupo.
    #[cfg(windows)]
    pub fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1);
        }
    }

    #[cfg(not(windows))]
    pub fn terminate(&self) {}
}

#[cfg(windows)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

/// Executa um programa e devolve (sucesso, stdout, stderr) — para chamadas curtas.
pub async fn run_capture(
    program: &Path,
    args: &[&str],
    timeout: std::time::Duration,
) -> std::io::Result<(bool, String, String)> {
    let child = command(program, args).spawn()?;
    let group = ProcessGroup::new()?;
    let _ = group.assign(&child);
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(out) => {
            let out = out?;
            Ok((
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).into_owned(),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        }
        Err(_) => {
            group.terminate();
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "tempo limite excedido"))
        }
    }
}
