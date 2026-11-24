> 🌐 **语言：** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · **中文** · [العربية](Home-ar) · [فارسی](Home-fa)

# 欢迎来到 Vessel Wiki

**Vessel** 是一款从零使用 **Rust**、**Tauri v2** 和 **Vue 3 / TypeScript** 构建的**本地优先（Local-First）**、注重隐私的跨平台流媒体与媒体中心。

---

## 核心理念

- **完全本地运行，零服务器成本：** 无需账号、无遥测数据。插件沙盒、SQLite 数据库（`WAL` + `FTS5`）、本地 HTTP/HLS 代理（`127.0.0.1`）、离线下载和元数据均在您的设备上运行。
- **Rust 核心 + 模块化界面：** 核心业务逻辑全部封装在跨平台 Rust Crates（`vessel-engine`）中，保持桌面端以及未来 Android / Android TV（`UniFFI`）外壳轻量高效。
- **域名级安全隔离：** TypeScript/JavaScript 插件在独立的 QuickJS 沙盒（`32 MiB` 内存限制、`15 秒`超时）中运行，并经过 DNS 解析后的 SSRF 防护与用户域名授权提示。

---

## 核心功能 (`v0.1.3`)

- **全屏播放器 (`/watch`)：** 支持 HLS (`hls.js`)、MPEG-TS (`mpegts.js`)、MP4/WebM 及通过 JSON IPC 调用外部 **`mpv`**；支持**多音轨切换**、**外挂/内封字幕**（`.srt/.vtt/.ass/.ssa`、`-500ms / +500ms` 微调及实时样式预览）、**返回详情页**按钮，以及自动跳过 **片头 / 回顾 / 片尾 / 预告**（`Plugin > TheIntroDB > IntroDB > AniSkip`）。
- **智能“继续观看”：** 播完一集后自动切换至下一集，自动隐藏已看完的电影与完结季；支持右键／长按菜单与手动进度编辑器（总时长自动读取当前集真实时长）。
- **4 级离线下载库：** 树形分组 **作品 → 季 → 集 → 视频源**（电影为 **作品 → 视频源**），支持 `8` 线程并行 HLS AES-128 与 HTTP `Range` 下载、同步下载音轨和字幕、即时暂停/删除以及拖拽调整下载队列顺序。
- **直播电视（IPTV & EPG）：** 支持 M3U+ 与 Xtream Codes、XMLTV 节目单（**正在播放 / 即将播放**）、SQLite FTS5 秒级搜索及专属 **直播插件** 标签页。
- **个性化配置：** 内置 **12 种语言**、**5 款主题** + `.theme.vessel` 导入、**6 种导航布局**（含悬浮底部栏）与 `.backup.vessel` 便携备份。

---

## 快捷键与触控板手势

| 操作 | 键盘 | 触控板手势 |
| :--- | :--- | :--- |
| 播放 / 暂停 | `空格` 或 `K` | — |
| 快退 / 快进 `10秒` | `←` / `J` 与 `→` / `L` | 双指水平滑动 |
| 音量 `+5%` / `-5%` | `↑` / `↓` | 双指垂直滑动 |
| 静音 / 全屏 | `M` / `F` | 双指捏合（`Ctrl + 滚轮`）切换画面填充 |

> 如在 macOS 上滑动方向相反，请在 **设置 → 播放与隐私** 中开启 **自然触控板滚动**。

---

## 快速安装 ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (Apple Silicon & Intel `.dmg`)：** 将 `Vessel.app` 拖入 *应用程序* 文件夹，并在终端执行一次：
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`)：** 如遇 SmartScreen 提示，请点击 **更多信息 → 仍要运行**。

---

## 完整英文技术文档

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
