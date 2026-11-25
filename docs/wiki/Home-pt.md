> 🌐 **Idiomas:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · **Português** · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Bem-vindo à Wiki do Vessel

O **Vessel** é um centro de mídia e streaming **local-first**, multiplataforma e focado em privacidade, construído do zero em **Rust**, **Tauri v2** e **Vue 3 / TypeScript**.

---

## Princípios Fundamentais

- **100% Local e Custo Zero:** Sem contas nem telemetria. Extensões, banco SQLite (`WAL` + `FTS5`), proxy HTTP/HLS (`127.0.0.1`), downloads offline e metadados rodam diretamente no seu dispositivo.
- **Núcleo em Rust + Interface Modular:** Toda a lógica fica nos crates Rust (`vessel-engine`), mantendo leves os apps de Desktop e futuros clientes Android / Android TV (`UniFFI`).
- **Segurança por Domínio:** Plugins TypeScript/JavaScript executam em sandboxes QuickJS isoladas (`32 MiB` de heap, limite de `15s`) com proteção SSRF e confirmação de permissões pelo usuário.

---

## Recursos Principais (`v0.1.45`)

- **Player em Tela Cheia (`/watch`):** Suporte para HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM e envio para **`mpv`** externo via JSON IPC; troca de **Faixas de Áudio**, **Legendas** (`.srt/.vtt/.ass/.ssa`) com sincronia (`-500ms / +500ms`) e pré-visualização, botão direto **Ir para Detalhes** e pulo automático de **Intro / Resumo / Créditos / Prévia** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Continuar Assistindo Inteligente:** Avança automaticamente para o próximo episódio após concluir o atual, oculta filmes/finais finalizados, oferece menu de contexto (botão direito / toque longo) e editor manual.
- **Downloads Offline em 4 Níveis:** Árvore **Conteúdo → Temporada → Episódio → Streams** (e **Conteúdo → Streams** para filmes), downloads HLS paralelos (`8` workers AES-128) + HTTP `Range`, áudio e legendas incluídos, pausa/exclusão instantânea e fila reordenável.
- **TV ao Vivo (IPTV & EPG):** Listas M3U+ e Xtream Codes, guia XMLTV (**Agora / Próximo**), busca instantânea SQLite FTS5 e aba **Plugins Live TV**.
- **Personalização:** **12 idiomas**, **5 temas nativos** + `.theme.vessel`, **6 layouts de navegação** e backups `.backup.vessel`.

---

## Atalhos e Gestos no Touchpad

| Ação | Teclado | Touchpad |
| :--- | :--- | :--- |
| Reproduzir / Pausar | `Espaço` ou `K` | — |
| Avançar / Voltar `10s` | `→` / `L` e `←` / `J` | Deslize horizontal com dois dedos |
| Volume `+5%` / `-5%` | `↑` / `↓` | Deslize vertical com dois dedos |
| Mudo / Tela cheia | `M` / `F` | Pinça (`Ctrl + Scroll`) alterna Fit/Fill |

---

## Instalação Rápida ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon & Intel):** Arraste `Vessel.app` para *Aplicativos* e execute uma vez no Terminal:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** No aviso do SmartScreen, clique em **Mais informações → Executar assim mesmo**.

---

## Documentação Completa (Inglês / Referência Principal)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
