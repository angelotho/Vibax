//! Validação de endereços informados pelo usuário antes de qualquer uso.
//!
//! Regras: apenas http/https, sem credenciais embutidas, com host, tamanho
//! limitado e sem caracteres de controle. A URL validada é sempre passada ao
//! yt-dlp após `--`, de modo que nunca possa ser interpretada como opção.

use serde::Serialize;
use url::Url;

use crate::error::{AppError, AppResult, ErrorCode};

const MAX_URL_LEN: usize = 2048;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UrlCheck {
    pub valid: bool,
    pub normalized: Option<String>,
    pub reason: Option<String>,
}

/// Normaliza e valida uma URL. Retorna a forma canônica (`Url::to_string`).
pub fn validate(input: &str) -> AppResult<String> {
    let trimmed = input
        .trim()
        .trim_matches(|c| c == '<' || c == '>' || c == '"' || c == '\'')
        .trim();

    let invalid = |msg: &str| AppError::new(ErrorCode::InvalidUrl, msg.to_string());

    if trimmed.is_empty() {
        return Err(invalid("Cole o endereço de um vídeo para continuar."));
    }
    if trimmed.len() > MAX_URL_LEN {
        return Err(invalid("O endereço é longo demais para ser válido."));
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid("O endereço contém espaços ou caracteres inválidos."));
    }

    // Aceita "youtube.com/watch?v=..." sem esquema, assumindo https.
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else if looks_like_host(trimmed) {
        format!("https://{trimmed}")
    } else {
        return Err(invalid("Isso não parece um endereço da web. Use um link que comece com https://"));
    };

    let url = Url::parse(&candidate)
        .map_err(|_| invalid("Não foi possível entender este endereço. Confira se ele foi copiado por completo."))?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(invalid("Apenas endereços http:// e https:// são aceitos.")),
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("Endereços com usuário ou senha embutidos não são aceitos."));
    }
    let host = url.host_str().unwrap_or_default();
    if host.is_empty() {
        return Err(invalid("O endereço não tem um domínio válido."));
    }
    if !host.contains('.') && host != "localhost" && url.host().map(|h| matches!(h, url::Host::Domain(_))).unwrap_or(true) {
        return Err(invalid("O domínio do endereço parece incompleto."));
    }

    Ok(url.to_string())
}

pub fn check(input: &str) -> UrlCheck {
    match validate(input) {
        Ok(n) => UrlCheck { valid: true, normalized: Some(n), reason: None },
        Err(e) => UrlCheck { valid: false, normalized: None, reason: Some(e.message) },
    }
}

/// Extrai URLs válidas (sem duplicatas, na ordem) de um texto com várias linhas.
pub fn extract_many(text: &str) -> (Vec<String>, Vec<String>) {
    let mut ok: Vec<String> = Vec::new();
    let mut bad = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        match validate(t) {
            Ok(u) => {
                if !ok.contains(&u) {
                    ok.push(u)
                }
            }
            Err(_) => bad.push(t.to_string()),
        }
    }
    (ok, bad)
}

fn looks_like_host(s: &str) -> bool {
    let host = s.split(['/', '?', '#']).next().unwrap_or("");
    host.contains('.')
        && !host.starts_with('.')
        && !host.starts_with('-')
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_regular_urls() {
        assert_eq!(
            validate("https://www.youtube.com/watch?v=aqz-KE-bpKQ").unwrap(),
            "https://www.youtube.com/watch?v=aqz-KE-bpKQ"
        );
        assert_eq!(validate("  youtu.be/aqz-KE-bpKQ ").unwrap(), "https://youtu.be/aqz-KE-bpKQ");
        assert!(validate("<https://vimeo.com/1084537>").is_ok());
    }

    #[test]
    fn rejects_dangerous_or_invalid() {
        for bad in [
            "",
            "file:///C:/Windows/System32/cmd.exe",
            "javascript:alert(1)",
            "--exec calc.exe",
            "-o C:/x https://a.com",
            "https://user:pass@example.com/v",
            "ftp://example.com/video.mp4",
            "hello world",
            "https://exa mple.com",
            "notaurl",
        ] {
            assert!(validate(bad).is_err(), "deveria rejeitar: {bad}");
        }
        let long = format!("https://a.com/{}", "x".repeat(3000));
        assert!(validate(&long).is_err());
    }

    #[test]
    fn extracts_many_unique() {
        let (ok, bad) = extract_many(
            "https://a.com/1\nhttps://a.com/1, https://b.com/2;lixo\n\n  youtu.be/abc",
        );
        assert_eq!(ok, vec!["https://a.com/1", "https://b.com/2", "https://youtu.be/abc"]);
        assert_eq!(bad, vec!["lixo"]);
    }
}
