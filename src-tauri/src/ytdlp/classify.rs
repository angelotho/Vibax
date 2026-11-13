//! Converte mensagens de erro do yt-dlp/FFmpeg em erros claros, em português.

use crate::error::{AppError, ErrorCode};

pub fn classify(error_lines: &[String], stderr_tail: &str) -> AppError {
    let primary = error_lines.last().cloned().unwrap_or_else(|| {
        stderr_tail
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string()
    });
    let hay = format!("{}\n{}", error_lines.join("\n"), stderr_tail).to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| hay.contains(n));

    let (code, msg) = if has(&["no space left", "errno 28", "winerror 112", "not enough space", "disk full"]) {
        (ErrorCode::DiskFull, "Não há espaço suficiente no disco de destino. Libere espaço ou escolha outra pasta.")
    } else if has(&["permission denied", "errno 13", "winerror 5]", "access is denied", "acesso negado"]) {
        (ErrorCode::Permission, "Sem permissão para gravar na pasta de destino. Escolha outra pasta nas configurações.")
    } else if has(&["ffmpeg not found", "ffprobe not found", "ffmpeg is not installed", "no js runtime", "js runtime"]) {
        (ErrorCode::ToolMissing, "Um componente interno do Vibax não foi encontrado. Reinstale o aplicativo.")
    } else if has(&["drm", "widevine", "playready"]) {
        (ErrorCode::Drm, "Este conteúdo é protegido (DRM) e não pode ser baixado.")
    } else if has(&["unsupported url"]) {
        (ErrorCode::Unsupported, "Este endereço não é de uma fonte compatível com o Vibax.")
    } else if has(&[
        "sign in to confirm",
        "login required",
        "requires authentication",
        "private video",
        "members-only",
        "join this channel",
        "this video is private",
        "age-restricted",
        "confirm your age",
        "inappropriate for some users",
        "--cookies",
        "use --username",
    ]) {
        (
            ErrorCode::AuthRequired,
            "Este conteúdo exige login ou verificação de conta. O Vibax não usa contas, cookies nem credenciais, então ele não pode ser baixado.",
        )
    } else if has(&["not available in your country", "geo restrict", "geo-restrict", "blocked it in your country"]) {
        (ErrorCode::GeoBlocked, "Este conteúdo não está disponível na sua região.")
    } else if has(&["this live event", "is live", "premieres in", "live stream"]) {
        (ErrorCode::Live, "Transmissões ao vivo ou agendadas não são suportadas.")
    } else if has(&["requested format is not available", "no video formats found", "format is not available"]) {
        (ErrorCode::NoFormats, "A qualidade escolhida não está mais disponível. Analise o link novamente e escolha outra opção.")
    } else if has(&["http error 403", "403: forbidden"]) {
        (
            ErrorCode::Forbidden,
            "A fonte recusou o download (HTTP 403). Atualize o mecanismo de download em Configurações e tente novamente.",
        )
    } else if has(&[
        "video unavailable",
        "video is unavailable",
        "is not available",
        "has been removed",
        "no longer available",
        "http error 404",
        "does not exist",
        "not found",
        "this video has been",
    ]) {
        (ErrorCode::Unavailable, "O vídeo não está disponível: pode ter sido removido ou o link está incorreto.")
    } else if has(&[
        "getaddrinfo",
        "errno 11001",
        "errno 11002",
        "name or service not known",
        "failed to resolve",
        "timed out",
        "connection reset",
        "connection refused",
        "connection aborted",
        "network is unreachable",
        "unable to download webpage",
        "urlopen error",
        "remote end closed",
        "ssl:",
        "http error 5",
        "incompleteread",
        "unable to download",
        "got error",
    ]) {
        (ErrorCode::Network, "Falha de conexão. Verifique sua internet e tente novamente.")
    } else if has(&["ffmpeg", "ffprobe", "postprocessing", "conversion failed", "merging", "muxer"]) {
        (ErrorCode::Processing, "Falha ao processar o arquivo (combinação ou conversão). Tente outro formato.")
    } else {
        (ErrorCode::Internal, "O download falhou por um motivo inesperado.")
    };

    AppError::new(code, msg).with_detail(primary.chars().take(600).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> ErrorCode {
        classify(&[s.to_string()], "").code
    }

    #[test]
    fn maps_common_errors() {
        assert_eq!(c("[generic] Unsupported URL: https://example.com/"), ErrorCode::Unsupported);
        assert_eq!(c("[youtube] abc: Video unavailable"), ErrorCode::Unavailable);
        assert_eq!(c("[youtube] abc: Sign in to confirm you're not a bot. Use --cookies-from-browser"), ErrorCode::AuthRequired);
        assert_eq!(c("[youtube] abc: Private video"), ErrorCode::AuthRequired);
        assert_eq!(
            c("Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>"),
            ErrorCode::Network
        );
        assert_eq!(c("[Errno 28] No space left on device"), ErrorCode::DiskFull);
        assert_eq!(c("[Errno 13] Permission denied: 'C:\\\\Windows\\\\x.part'"), ErrorCode::Permission);
        assert_eq!(c("This video is DRM protected"), ErrorCode::Drm);
        assert_eq!(c("Postprocessing: Conversion failed!"), ErrorCode::Processing);
        assert_eq!(c("unable to download video data: HTTP Error 403: Forbidden"), ErrorCode::Forbidden);
        assert_eq!(c("Requested format is not available"), ErrorCode::NoFormats);
        assert_eq!(c("algo estranho"), ErrorCode::Internal);
    }
}
