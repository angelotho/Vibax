# Changelog

Todas as mudanças relevantes do Vibax são documentadas neste arquivo.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto adota o
[Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [1.0.2] — 2026-11-15

### Apoio ao projeto
- Card "Curtiu o Vibax?" no rodapé da barra lateral, com atalho para doação via Pix. Pode ser dispensado e volta
  após 14 dias; enquanto isso fica um botão discreto "Apoie o projeto".
- Janela de doação com QR Code Pix (gerado localmente, sem internet), botão "Pix copia e cola" e cópia da chave.

## [1.0.0] — 2026-11-13

Primeira versão pública.

### Análise de links
- Prévia com miniatura, título, canal, duração e origem do conteúdo.
- Qualidades listadas a partir dos formatos realmente oferecidos pela fonte (144p a 4K/8K), com tamanho estimado
  de cada opção; nenhuma resolução inexistente é exibida.
- Classificação correta de vídeos verticais e com bordas recortadas (pelo lado menor do quadro).
- Suporte a playlists: lista os itens e adiciona todos à fila com a mesma preferência.
- Validação de endereços (somente http/https, sem credenciais embutidas) com mensagens claras.
- Sugestão do link copiado na área de transferência ao voltar para o aplicativo (desativável).

### Downloads
- Formatos de vídeo MP4 (prioridade, com H.264 sempre que disponível), WebM e MKV.
- Extração de áudio em MP3 (128, 192, 256 ou 320 kbps) ou M4A (AAC sem recompressão quando possível).
- Combinação automática de vídeo e áudio separados pelo FFmpeg, sem perda de qualidade.
- Preferência por áudio estéreo e por fluxos adaptativos, evitando formatos legados que o YouTube costuma recusar.
- Gravação de título, capítulos e capa no arquivo (desativável).
- Nova tentativa automática (até 3 vezes) para falhas de rede e recusas intermitentes (HTTP 403), aproveitando o
  que já foi baixado.

### Fila
- Downloads simultâneos configuráveis (1 a 6); os demais aguardam vaga.
- Estados: aguardando, analisando, baixando, processando, pausado, concluído, cancelado e erro.
- Porcentagem, bytes baixados, tamanho, velocidade e tempo restante em tempo real.
- Pausar e retomar de onde parou (HTTP Range), cancelar (remove os arquivos parciais), tentar de novo e remover.
- Vários links de uma vez.
- Downloads interrompidos pelo fechamento do aplicativo voltam como pausados na próxima abertura.
- Confirmação ao fechar a janela com downloads em andamento.

### Histórico e configurações
- Histórico local pesquisável (tolerante a acentos), com filtro por vídeo/áudio, abrir arquivo, mostrar na pasta,
  baixar novamente e detecção de arquivos movidos ou excluídos.
- Configurações persistentes: pasta padrão, tipo, qualidade e formatos preferidos, downloads simultâneos, tema
  (escuro, claro ou igual ao Windows), notificações do Windows e detecção de links copiados.

### Componentes e segurança
- yt-dlp, FFmpeg e Deno incluídos no instalador; nada precisa ser instalado à parte.
- Atualização do yt-dlp a partir da release oficial, com verificação SHA-256, extração protegida e opção de voltar à
  versão original.
- Processos externos executados sem shell, com argumentos validados e encerramento da árvore inteira via Job
  Objects do Windows.
- Permissões mínimas do Tauri e política de segurança de conteúdo restrita.
- Sem contas, anúncios, rastreadores, cookies ou credenciais. Conteúdo com DRM, login ou outras restrições não é
  baixado.

### Instalação
- Instalador NSIS para Windows 10/11 (x64), por usuário, sem exigir administrador, com atalhos no menu Iniciar e
  na área de trabalho e desinstalador.

[1.0.0]: https://github.com/angelotho/Vibax/releases/tag/v1.0.0
