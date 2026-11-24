> 🌐 **言語:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · **日本語** · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Vessel Wiki へようこそ

**Vessel** は、**Rust**・**Tauri v2**・**Vue 3 / TypeScript** で一から構築された、**ローカルファースト（Local-First）**かつプライバシー重視のクロスプラットフォーム・ストリーミング＆メディアセンターです。

---

## 基本設計

- **完全ローカル実行＆サーバー費用ゼロ:** アカウント不要・テレメトリーなし。拡張機能、SQLite (`WAL` + `FTS5`)、ローカル HTTP/HLS プロキシ (`127.0.0.1`)、オフラインダウンロード、メタデータ処理をすべて端末内で完結します。
- **Rust コア + モジュール式 UI:** すべてのロジックを UI 非依存の Rust クレート（`vessel-engine`）に集約し、デスクトップ版と将来の Android / Android TV 版（`UniFFI`）を軽量に保ちます。
- **ドメイン単位のセキュリティ:** TypeScript/JavaScript プラグインは隔離された QuickJS サンドボックス（メモリ上限 `32 MiB`、タイムアウト `15秒`）で実行され、DNS 解決後の SSRF 検証とユーザー許可プロンプトを通過します。

---

## 主な機能 (`v0.1.4`)

- **CloudStream 風フルスクリーンプレイヤー (`/watch`):** HLS (`hls.js`)・MPEG-TS (`mpegts.js`)・MP4/WebM 再生および JSON IPC 経由の外部 **`mpv`** 連携。**音声トラック切替**、**字幕**（`.srt/.vtt/.ass/.ssa` + `-500ms / +500ms` 同期・リアルタイムプレビュー）、**詳細ページへ移動**ボタン、**イントロ / あらすじ / エンディング / 予告スキップ**（`Plugin > TheIntroDB > IntroDB > AniSkip`）に対応。
- **スマート「視聴を続ける」:** エピソード完了時に自動で次話へ進行し、視聴済み映画や最終話を非表示にします。右クリック／長押しメニューや、話数と視聴位置を手動変更できるエディターも搭載。
- **4階層オフラインダウンロード:** **作品 → シーズン → エピソード → ストリーム**（映画は **作品 → ストリーム**）のツリー表示、8ワーカー並列 AES-128 HLS・HTTP `Range` ダウンロード、音声/字幕バンドル、即時一時停止/削除、ドラッグ＆ドロップ対応キュー。
- **ライブ TV（IPTV & EPG）:** M3U+ / Xtream Codes、XMLTV 番組表（現在／次の番組）、SQLite FTS5 高速検索、**ライブ TV プラグイン**タブ。
- **カスタマイズ:** **12言語**対応、**5つの内蔵テーマ**＋`.theme.vessel` インポート、**6種類のナビゲーション配置**、`.backup.vessel` バックアップ。

---

## ショートカット＆タッチパッド操作

| 操作 | キーボード | タッチパッド |
| :--- | :--- | :--- |
| 再生 / 一時停止 | `Space` または `K` | — |
| 10秒 戻る / 進む | `←` / `J` ・ `→` / `L` | 2本指 横スワイプ |
| 音量 `+5%` / `-5%` | `↑` / `↓` | 2本指 縦スワイプ |
| ミュート / 全画面 | `M` / `F` | ピンチ（`Ctrl + スクロール`）で Fit/Fill 切替 |

> macOS のナチュラルスクロールと方向が逆の場合は **設定 → 再生とプライバシー → ナチュラルタッチパッドスクロール** を有効にしてください。

---

## インストール ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon / Intel):** `Vessel.app` を *アプリケーション* にドラッグ後、ターミナルで1度だけ実行:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** SmartScreen 警告が出た場合は **詳細情報 → 実行** をクリックしてください。

---

## 詳細ドキュメント（英語・メイン）

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
