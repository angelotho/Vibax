# Componentes de terceiros

O código do Vibax é licenciado sob a [PolyForm Noncommercial 1.0.0](LICENSE.md). O instalador para Windows
também distribui os programas abaixo. Eles são **programas independentes**, executados pelo Vibax como processos
separados, e **cada um mantém a sua própria licença** — a restrição não comercial do Vibax não se aplica a eles.

Os textos completos das licenças acompanham o instalador em `tools\licenses\`.

## yt-dlp

- Função: análise de links e download dos fluxos de mídia.
- Projeto: <https://github.com/yt-dlp/yt-dlp>
- Versão distribuída no Vibax 1.0.0: `2026.10.26` (build oficial `yt-dlp_win.zip`)
- Licença: [The Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE) (domínio público). A build
  oficial para Windows inclui componentes com outras licenças, listados em
  <https://github.com/yt-dlp/yt-dlp/blob/master/THIRD_PARTY_LICENSES.txt>.

## FFmpeg

- Função: combinação de vídeo e áudio, conversão de áudio e gravação de metadados.
- Projeto: <https://ffmpeg.org>
- Versão distribuída no Vibax 1.0.0: `N-128867-g21ae927dc6-20261112`, build `win64-gpl-shared` do projeto
  [yt-dlp/FFmpeg-Builds](https://github.com/yt-dlp/FFmpeg-Builds).
- Licença: **GNU General Public License v3** (a build inclui bibliotecas GPL).
- Código-fonte correspondente:
  - FFmpeg no commit exato da build:
    <https://github.com/FFmpeg/FFmpeg/tree/21ae927dc6fd9b7cf8be3b54d620c1f707c83095>
  - Receita de compilação e versões das bibliotecas incluídas:
    <https://github.com/yt-dlp/FFmpeg-Builds/tree/2e105eceb226411f9a56c821306c2c2d00b7a389>
  - Uma cópia do código-fonte do FFmpeg desse commit também é publicada como anexo da Release v1.0.0.
- O FFmpeg não foi modificado pelo Vibax. Você pode substituí-lo por outra build compatível na pasta `tools`.

## Deno

- Função: runtime JavaScript usado pelo yt-dlp para resolver os desafios do YouTube (sem acesso a rede ou disco).
- Projeto: <https://deno.com> — <https://github.com/denoland/deno>
- Versão distribuída no Vibax 1.0.0: `2.9.8`
- Licença: [MIT](https://github.com/denoland/deno/blob/main/LICENSE.md)

## Bibliotecas incorporadas ao aplicativo

O aplicativo é construído com bibliotecas de código aberto declaradas em `package.json` e
`src-tauri/Cargo.toml`, majoritariamente sob licenças MIT, Apache-2.0 ou BSD, entre elas:
[Tauri](https://tauri.app) (MIT/Apache-2.0), [React](https://react.dev) (MIT),
[Radix UI](https://www.radix-ui.com) e [shadcn/ui](https://ui.shadcn.com) (MIT),
[Tailwind CSS](https://tailwindcss.com) (MIT), [Lucide](https://lucide.dev) (ISC),
[Zustand](https://github.com/pmndrs/zustand) (MIT), [Sonner](https://sonner.emilkowal.ski) (MIT),
[rusqlite](https://github.com/rusqlite/rusqlite) / SQLite (MIT / domínio público),
[Tokio](https://tokio.rs) (MIT) e [reqwest](https://github.com/seanmonstar/reqwest) (MIT/Apache-2.0).
A fonte [Onest](https://github.com/simpals/onest) é distribuída sob a SIL Open Font License 1.1.
