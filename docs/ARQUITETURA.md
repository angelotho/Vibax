# Vibax — documentação técnica

Arquitetura, decisões de projeto, segurança, testes e procedimentos de build.
Para a apresentação do projeto, veja o [README](../README.md).

## Arquitetura

```
┌──────────────────────── Interface (React + TS) ───────────────────────┐
│ pages/      Baixar · Fila · Histórico · Configurações                  │
│ components/ layout, download (prévia, seletores), queue, common, ui    │
│ store/      Zustand: fila (eventos), configurações, navegação          │
│ lib/api.ts  única porta para o backend (invoke + eventos)              │
└───────────────▲─────────────────────────────────────┬─────────────────┘
        eventos │ job-updated / job-removed           │ comandos (invoke)
┌───────────────┴──────────── Backend Rust (Tauri 2) ─▼─────────────────┐
│ commands.rs   fronteira: validação de toda entrada                     │
│ downloads/    Manager: fila, agendador, pausa/retomada, recuperação    │
│ ytdlp/        info (formatos reais) · spec (argumentos) · progress     │
│               (leitura de progresso) · classify (erros em português)   │
│ storage/      SQLite (fila + histórico) · settings.json (atômico)      │
│ tools.rs      localização dos binários empacotados                     │
│ updater.rs    atualização verificada do yt-dlp                         │
│ process.rs    processos sem shell + Job Objects do Windows             │
│ system.rs     espaço livre, permissão de escrita, pastas temporárias   │
└───────────────┬────────────────────────────────────────────────────────┘
                │ argumentos em vetor, URL sempre após "--"
        yt-dlp (onedir) ── Deno (desafios JS do YouTube) ── FFmpeg/FFprobe
```

### Fluxo de um download

1. A URL é validada (`url_guard.rs`: só http/https, sem credenciais, tamanho limitado).
2. `yt-dlp -J` devolve os formatos; `ytdlp/info.rs` agrupa por resolução nominal (pelo lado menor do quadro,
   então vídeos verticais também são classificados corretamente) e escolhe, por contêiner, o melhor fluxo
   de vídeo + áudio. Só aparecem resoluções que existem.
3. Ao clicar em **Baixar**, a escolha (IDs de formato validados) vai para a fila. O `Manager` respeita o limite de
   simultâneos, verifica pasta, permissão e espaço livre, e executa o yt-dlp com arquivos temporários em
   `<destino>\.vibax-tmp\<id>` (mesmo disco: a movimentação final é instantânea).
4. O progresso é lido de linhas marcadas (`--progress-template`/`--print`) em stdout **e** stderr e enviado à
   interface no máximo a cada 200 ms.
5. Concluído: o arquivo vai para o destino, o histórico é gravado e (conforme a preferência) uma notificação
   do Windows é exibida.

### Principais decisões técnicas

| Decisão | Motivo |
| --- | --- |
| **Tauri 2 (estável)** | Tauri 3 ainda está em alpha. O app final pesa poucos MB além dos componentes. |
| **yt-dlp "onedir" (`yt-dlp_win.zip`)** | O `.exe` onefile se extrai em `%TEMP%` a cada execução (lento) e deixa lixo quando o processo é encerrado no cancelamento. |
| **Deno empacotado** | O yt-dlp atual exige um runtime JavaScript para resolver os desafios do YouTube; o Deno é o recomendado oficialmente e roda os scripts sem acesso a rede/disco. |
| **FFmpeg "gpl-shared" de `yt-dlp/FFmpeg-Builds`** | Build mantida pelo projeto yt-dlp, com correções específicas; a variante *shared* reduz o tamanho total. |
| **Componentes como `resources`** (não `externalBin`) | O yt-dlp onedir e as DLLs do FFmpeg são pastas, não um único executável. |
| **Job Objects do Windows** | Cancelar encerra a árvore inteira (yt-dlp → ffmpeg/deno); se o Vibax fechar inesperadamente, nenhum processo fica órfão. |
| **Pausa = encerrar mantendo `.part`** | O yt-dlp retoma com HTTP Range a partir do arquivo parcial. Testado (o servidor de teste confere o Range). |
| **SQLite (rusqlite, embutido)** | Fila e histórico sobrevivem a quedas; tarefas interrompidas voltam como "pausadas". |
| **Nova tentativa automática (até 3×)** | O YouTube às vezes recusa URLs de CDN com HTTP 403 de forma intermitente; uma nova extração resolve e reaproveita os parciais. |
| **Estéreo e fluxos adaptativos preferidos** | O itag 18 (progressivo) do YouTube hoje costuma falhar com 403, e áudio 5.1 tem compatibilidade ruim. |
| **`--ignore-config --no-plugin-dirs`** | Configurações/plugins do yt-dlp instalados na máquina não podem alterar o comportamento (ex.: injetar cookies). |

### Segurança

- Nenhum shell: processos recebem argumentos em vetor; a URL vem sempre depois de `--`.
- IDs de formato aceitam só `[A-Za-z0-9._-]`; bitrate e qualidade vêm de listas fixas.
- Permissões mínimas do Tauri (`src-tauri/capabilities/default.json`): a interface não tem acesso a shell, sistema
  de arquivos ou abertura arbitrária de caminhos — abrir arquivo/pasta é feito pelo backend a partir do ID da tarefa.
- CSP restrita; protocolo `asset:` limitado à pasta de miniaturas do app.
- Atualização do yt-dlp: apenas `github.com/yt-dlp/yt-dlp` via HTTPS, verificação SHA-256 contra o `SHA2-256SUMS`
  da mesma release, extração protegida contra *path traversal*, e ativação só depois que a nova versão responde a
  `--version`. A versão do instalador nunca é alterada e pode ser restaurada em Configurações.

---

## Requisitos de desenvolvimento

- Windows 10/11 x64
- [Node.js](https://nodejs.org) 22+ (testado com 24) e npm
- [Rust](https://rustup.rs) estável (testado com 1.100, toolchain `x86_64-pc-windows-msvc`)
- Visual Studio 2022 Build Tools com "Desenvolvimento para desktop com C++" e Windows 11 SDK
- WebView2 (já incluso no Windows 11)

## Comandos

```powershell
npm install
npm run fetch-binaries      # baixa yt-dlp, FFmpeg e Deno oficiais e confere o SHA-256 (src-tauri/tools)
npm run tauri dev           # app em modo desenvolvimento
npm test                    # testes do frontend (Vitest)
cd src-tauri; cargo test    # testes unitários e de integração do backend
npm run tauri build         # gera o instalador
```

> Abra o terminal em `D:\...` (letra maiúscula). Com `d:\...` o Vitest pode carregar módulos duas vezes no
> Windows; o `vite.config.ts` já normaliza isso para o servidor de desenvolvimento.

`npm run fetch-binaries -- --force` baixa novamente as versões mais recentes.

### Testes

- **Unitários (Rust, 20)**: validação de URL (inclusive tentativas de injeção), seleção de formatos, montagem de
  argumentos, leitura de progresso, classificação de erros, configurações, espaço em disco.
- **Integração (Rust, 12)**: usam os binários empacotados com o `PATH` reduzido ao `System32` (prova de que nada
  depende de programas instalados na máquina). Um servidor HTTP local serve vídeos gerados pelo próprio FFmpeg,
  com suporte a Range e velocidade limitada. Cobrem: análise, download completo com mesclagem e metadados, estados
  percorridos, histórico persistido e recarregado, arquivo excluído detectado, cancelamento (parciais removidos),
  pausa/retomada com Range real, limite de simultâneos, pasta sem permissão, recuperação após fechar no meio do
  download, links inválidos/não suportados/inexistentes/rede indisponível e, na internet, o filme *Big Buck Bunny*
  (Blender Foundation, CC BY 3.0) em 360p MP4 e MP3, além do ciclo completo de atualização do yt-dlp (download oficial, SHA-256, ativação e restauração) e a listagem de uma playlist real. Use `VIBAX_SKIP_NETWORK=1` para pular os testes que acessam a
  internet.
- **Frontend (Vitest)**: formatação e detecção de links.

## Gerar o instalador

```powershell
npm run fetch-binaries
npm run tauri build
```

O instalador NSIS fica em `src-tauri\target\release\bundle\nsis\Vibax_<versão>_x64-setup.exe`. Ele instala por
usuário (sem exigir administrador), cria atalhos no menu Iniciar e na área de trabalho e inclui os componentes em
`<pasta de instalação>\tools`.

## Dados locais

| O quê | Onde |
| --- | --- |
| Fila e histórico | `%LOCALAPPDATA%\app.vibax.desktop\vibax.db` |
| Miniaturas | `%LOCALAPPDATA%\app.vibax.desktop\thumbs` |
| yt-dlp atualizado | `%LOCALAPPDATA%\app.vibax.desktop\tools` |
| Configurações | `%APPDATA%\app.vibax.desktop\settings.json` |
| Downloads (padrão) | `%USERPROFILE%\Downloads\Vibax` |

## Limitações conhecidas

- **Conteúdo com login, idade restrita, membros, DRM ou bloqueio regional não é baixado** — por decisão de projeto.
  O Vibax explica o motivo em vez de falhar silenciosamente.
- **Transmissões ao vivo** não são suportadas.
- **YouTube muda com frequência.** Se começar a falhar (ex.: HTTP 403 persistente), use *Configurações → Atualizar o
  yt-dlp*.
- **MP4 em 1440p/4K no YouTube é AV1** (não existe H.264 nessas resoluções); a interface avisa que players antigos
  podem não reproduzir. Não há recompressão para H.264 — seria lenta e perderia qualidade.
- **Pausa/retomada** depende de o servidor aceitar requisições parciais (HTTP Range). Os principais aceitam; em
  fontes que não aceitam, retomar recomeça o arquivo.
- O instalador não é assinado digitalmente; o SmartScreen pode exibir um aviso até que um certificado de assinatura
  de código seja configurado (`bundle.windows.certificateThumbprint`).
- A integridade das atualizações é verificada por SHA-256 do mesmo servidor oficial; a assinatura GPG do yt-dlp
  ainda não é verificada.
- O instalador tem ~110 MB, principalmente por causa do FFmpeg e do Deno.

## Licenças

Código do Vibax: [PolyForm Noncommercial 1.0.0](../LICENSE.md). Componentes de terceiros: veja
[THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md).
