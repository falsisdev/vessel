//! Desktop `mpv` window backend implementing `vessel_domain::PlayerBackend` via JSON IPC,
//! plus lazy dynamically-loaded `libmpv` (`embedded`) playback.

pub mod embedded;
pub mod ipc;
pub mod libmpv_ffi;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use vessel_domain::PlaybackStats;
use vessel_domain::{
    EventSink, PlaybackRequest, PlayerBackend, SubtitleFormat, SubtitleSource, SubtitleTrack,
    VesselError, VesselResult,
};

use ipc::MpvIpcClient;

const SOCKET_CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
const HOST_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(25);
const MAX_STDERR_CAPTURE_BYTES: usize = 4096;
static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// Returns `true` when running inside a Linux Flatpak sandbox.
fn is_flatpak_sandbox() -> bool {
    cfg!(target_os = "linux")
        && (Path::new("/.flatpak-info").exists() || std::env::var_os("FLATPAK_ID").is_some())
}

/// Computes a runtime directory accessible to both the Flatpak sandbox and host `mpv`.
///
/// Flatpak bind-mounts `$XDG_RUNTIME_DIR/app/$FLATPAK_ID` at the exact same path inside the
/// sandbox and on the host, whereas `/tmp` is private to the sandbox.
fn resolve_default_runtime_dir(
    flatpak: bool,
    xdg_runtime_dir: Option<&Path>,
    flatpak_id: Option<&str>,
    temp_dir: &Path,
    suffix: &str,
) -> PathBuf {
    if flatpak
        && let Some(runtime_base) = xdg_runtime_dir.filter(|p| !p.as_os_str().is_empty())
        && let Some(app_id) = flatpak_id.map(str::trim).filter(|s| !s.is_empty())
    {
        return runtime_base.join("app").join(app_id).join(suffix);
    }
    temp_dir.join(suffix)
}

/// Launch configuration for `MpvWindowBackend`.
#[derive(Debug, Clone)]
pub struct MpvConfig {
    /// Path or executable name of `mpv` (defaults to `"mpv"` in `$PATH`).
    pub binary_path: PathBuf,
    /// Additional CLI flags appended when spawning `mpv`.
    pub extra_args: Vec<String>,
    /// Directory where ephemeral IPC sockets and temporary inline subtitle files are stored.
    pub runtime_dir: PathBuf,
}

impl Default for MpvConfig {
    fn default() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seq = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
        let suffix = format!("vessel-mpv-{nanos}-{seq}");
        let xdg_runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        let flatpak_id = std::env::var("FLATPAK_ID").ok();
        let temp_dir = std::env::temp_dir();
        Self {
            binary_path: PathBuf::from("mpv"),
            extra_args: Vec::new(),
            runtime_dir: resolve_default_runtime_dir(
                is_flatpak_sandbox(),
                xdg_runtime.as_deref(),
                flatpak_id.as_deref(),
                &temp_dir,
                &suffix,
            ),
        }
    }
}

struct MpvProcessState {
    child: Option<Child>,
    ipc: Option<MpvIpcClient>,
    socket_path: Option<PathBuf>,
    temp_subtitles: Vec<PathBuf>,
}

/// Out-of-process `mpv` window backend (`MpvWindowBackend`) controlled over JSON IPC.
pub struct MpvWindowBackend {
    config: MpvConfig,
    event_sink: Option<Arc<dyn EventSink>>,
    state: Mutex<MpvProcessState>,
    sub_counter: AtomicU64,
}

impl std::fmt::Debug for MpvWindowBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MpvWindowBackend")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

fn subtitle_extension(format: SubtitleFormat) -> &'static str {
    match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Ass => "ass",
        SubtitleFormat::Ssa => "ssa",
        SubtitleFormat::Vtt => "vtt",
    }
}

/// Resolved process invocation strategy for `mpv` (native direct or Flatpak host bridge).
#[derive(Debug, Clone, PartialEq, Eq)]
enum MpvLaunchPlan {
    Direct(PathBuf),
    FlatpakHostBinary {
        spawn_bin: PathBuf,
        binary: String,
    },
    FlatpakHostApp {
        spawn_bin: PathBuf,
        app_id: String,
        runtime_dir: PathBuf,
    },
}

/// Resolves `mpv` executable even when macOS/Windows GUI bundles launch with a stripped `PATH`.
fn resolve_mpv_binary(configured: &Path) -> PathBuf {
    if configured != Path::new("mpv") || configured.is_absolute() {
        return configured.to_path_buf();
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(if cfg!(windows) { "mpv.exe" } else { "mpv" });
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    let mut extra_candidates: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        extra_candidates.push(PathBuf::from("/opt/homebrew/bin/mpv"));
        extra_candidates.push(PathBuf::from("/usr/local/bin/mpv"));
        extra_candidates.push(PathBuf::from("/Applications/mpv.app/Contents/MacOS/mpv"));
        if let Some(home) = std::env::var_os("HOME") {
            extra_candidates
                .push(PathBuf::from(home).join("Applications/mpv.app/Contents/MacOS/mpv"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        extra_candidates.push(PathBuf::from("/usr/bin/mpv"));
        extra_candidates.push(PathBuf::from("/usr/local/bin/mpv"));
        extra_candidates.push(PathBuf::from("/snap/bin/mpv"));
        extra_candidates.push(PathBuf::from("/home/linuxbrew/.linuxbrew/bin/mpv"));
        extra_candidates.push(PathBuf::from("/run/current-system/sw/bin/mpv"));
        if let Some(home) = std::env::var_os("HOME") {
            let h = PathBuf::from(home);
            extra_candidates.push(h.join(".linuxbrew/bin/mpv"));
            extra_candidates.push(h.join(".nix-profile/bin/mpv"));
        }
    }
    #[cfg(windows)]
    {
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            extra_candidates.push(PathBuf::from(pf).join("mpv/mpv.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            extra_candidates.push(PathBuf::from(local).join("Programs/mpv/mpv.exe"));
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            extra_candidates.push(PathBuf::from(profile).join("scoop/shims/mpv.exe"));
        }
    }

    extra_candidates
        .into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| configured.to_path_buf())
}

fn resolve_flatpak_spawn_binary() -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("flatpak-spawn");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    [
        Path::new("/usr/bin/flatpak-spawn"),
        Path::new("/bin/flatpak-spawn"),
        Path::new("/app/bin/flatpak-spawn"),
    ]
    .into_iter()
    .find(|p| p.is_file())
    .map(Path::to_path_buf)
}

const FLATPAK_APP_PROBE_PREFIX: &str = "__FLATPAK_APP__:";

fn parse_host_mpv_probe_output(raw: &str) -> Option<Result<String, String>> {
    let line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    if let Some(app_id) = line.strip_prefix(FLATPAK_APP_PROBE_PREFIX) {
        let trimmed = app_id.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Err(trimmed.to_owned()))
        }
    } else {
        Some(Ok(line.to_owned()))
    }
}

fn missing_mpv_error(is_flatpak: bool) -> VesselError {
    if is_flatpak {
        VesselError::Player(
            "mpv executable not found on the host system. Install mpv on your host (e.g. 'sudo apt install mpv', 'sudo dnf install mpv', 'sudo pacman -S mpv', or 'flatpak install flathub io.mpv.Mpv') to launch external playback."
                .into(),
        )
    } else if cfg!(target_os = "linux") {
        VesselError::Player(
            "mpv executable not found. Install mpv (e.g. 'sudo apt install mpv', 'sudo dnf install mpv', 'sudo pacman -S mpv', or 'flatpak install flathub io.mpv.Mpv') to launch external playback."
                .into(),
        )
    } else {
        VesselError::Player(
            "mpv executable not found. Install mpv (e.g. 'brew install mpv' on macOS or winget/scoop on Windows) to launch external playback."
                .into(),
        )
    }
}

async fn resolve_mpv_launch(config: &MpvConfig) -> VesselResult<MpvLaunchPlan> {
    if is_flatpak_sandbox() {
        let explicit_sandbox_bin =
            config.binary_path != Path::new("mpv") && config.binary_path.is_file();
        if explicit_sandbox_bin {
            return Ok(MpvLaunchPlan::Direct(config.binary_path.clone()));
        }

        let Some(spawn_bin) = resolve_flatpak_spawn_binary() else {
            return Err(VesselError::Player(
                "flatpak-spawn is unavailable inside the sandbox; grant org.freedesktop.Flatpak portal access or install mpv locally."
                    .into(),
            ));
        };

        let probe_script = r#"
if [ -n "$1" ] && [ "$1" != "mpv" ] && [ -x "$1" ]; then
  printf '%s\n' "$1"
  exit 0
fi
if command -v mpv >/dev/null 2>&1; then
  command -v mpv
  exit 0
fi
for p in \
  /usr/bin/mpv \
  /usr/local/bin/mpv \
  /snap/bin/mpv \
  /var/lib/snapd/snap/bin/mpv \
  /home/linuxbrew/.linuxbrew/bin/mpv \
  /run/current-system/sw/bin/mpv \
  "$HOME/.linuxbrew/bin/mpv" \
  "$HOME/.nix-profile/bin/mpv" \
  "/etc/profiles/per-user/${USER:-}/bin/mpv"
do
  if [ -x "$p" ]; then
    printf '%s\n' "$p"
    exit 0
  fi
done
if command -v flatpak >/dev/null 2>&1 && flatpak info io.mpv.Mpv >/dev/null 2>&1; then
  printf '__FLATPAK_APP__:io.mpv.Mpv\n'
  exit 0
fi
exit 1
"#;

        let mut probe_cmd = Command::new(&spawn_bin);
        probe_cmd
            .arg("--host")
            .arg("/bin/sh")
            .arg("-c")
            .arg(probe_script)
            .arg("vessel-mpv-probe")
            .arg(config.binary_path.to_string_lossy().as_ref())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = tokio::time::timeout(HOST_PROBE_TIMEOUT, probe_cmd.output())
            .await
            .map_err(|_| {
                VesselError::Player(
                    "timed out probing host mpv via flatpak-spawn".into(),
                )
            })?
            .map_err(|err| {
                VesselError::Player(format!(
                    "probe host mpv via {}: {err}",
                    spawn_bin.display()
                ))
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match parse_host_mpv_probe_output(&stdout) {
                Some(Ok(binary)) => {
                    return Ok(MpvLaunchPlan::FlatpakHostBinary { spawn_bin, binary });
                }
                Some(Err(app_id)) => {
                    return Ok(MpvLaunchPlan::FlatpakHostApp {
                        spawn_bin,
                        app_id,
                        runtime_dir: config.runtime_dir.clone(),
                    });
                }
                None => {}
            }
        }

        let stderr_text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if !stderr_text.is_empty() && output.status.code() != Some(1) {
            return Err(VesselError::Player(format!(
                "flatpak-spawn host mpv check failed: {stderr_text}"
            )));
        }
        return Err(missing_mpv_error(true));
    }

    Ok(MpvLaunchPlan::Direct(resolve_mpv_binary(
        &config.binary_path,
    )))
}

fn build_mpv_command(
    plan: &MpvLaunchPlan,
    socket_path: &Path,
    initial_title: Option<&str>,
    extra_args: &[String],
) -> (Command, String) {
    let mut cmd = match plan {
        MpvLaunchPlan::Direct(bin) => Command::new(bin),
        MpvLaunchPlan::FlatpakHostBinary { spawn_bin, binary } => {
            let mut c = Command::new(spawn_bin);
            c.arg("--host").arg(binary);
            c
        }
        MpvLaunchPlan::FlatpakHostApp {
            spawn_bin,
            app_id,
            runtime_dir,
        } => {
            let mut c = Command::new(spawn_bin);
            c.arg("--host")
                .arg("flatpak")
                .arg("run")
                .arg(format!("--filesystem={}", runtime_dir.display()))
                .arg(app_id);
            c
        }
    };

    cmd.arg("--idle=yes")
        .arg("--force-window=yes")
        .arg("--no-terminal")
        .arg(format!("--input-ipc-server={}", socket_path.display()));
    if let Some(title) = initial_title {
        cmd.arg(format!("--title={title}"));
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let display_target = match plan {
        MpvLaunchPlan::Direct(bin) => bin.display().to_string(),
        MpvLaunchPlan::FlatpakHostBinary { binary, .. } => {
            format!("flatpak-spawn --host {binary}")
        }
        MpvLaunchPlan::FlatpakHostApp { app_id, .. } => {
            format!("flatpak-spawn --host flatpak run {app_id}")
        }
    };
    (cmd, display_target)
}

impl MpvWindowBackend {
    /// Creates a desktop `mpv` window backend using `config` and forwarding state to `event_sink`.
    pub fn new(config: MpvConfig, event_sink: Option<Arc<dyn EventSink>>) -> Self {
        Self {
            config,
            event_sink,
            state: Mutex::new(MpvProcessState {
                child: None,
                ipc: None,
                socket_path: None,
                temp_subtitles: Vec::new(),
            }),
            sub_counter: AtomicU64::new(1),
        }
    }

    /// Attaches an already-listening `mpv` (or mock) JSON IPC socket path.
    pub async fn attach_ipc_socket(&self, socket_path: &Path) -> VesselResult<()> {
        let ipc = MpvIpcClient::connect(socket_path, self.event_sink.clone()).await?;
        let mut guard = self.state.lock().await;
        guard.ipc = Some(ipc);
        guard.socket_path = Some(socket_path.to_path_buf());
        Ok(())
    }

    async fn ensure_connected(&self, initial_title: Option<&str>) -> VesselResult<()> {
        let mut guard = self.state.lock().await;
        if let Some(child) = guard.child.as_mut()
            && let Ok(Some(_exit)) = child.try_wait()
        {
            guard.child = None;
            guard.ipc = None;
        }
        if guard.ipc.is_some() {
            return Ok(());
        }

        fs::create_dir_all(&self.config.runtime_dir).map_err(|err| {
            VesselError::Player(format!(
                "create mpv runtime dir {}: {err}",
                self.config.runtime_dir.display()
            ))
        })?;

        let seq = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
        let socket_path = self.config.runtime_dir.join(format!("mpv-{seq}.sock"));
        let _ = fs::remove_file(&socket_path);

        let plan = resolve_mpv_launch(&self.config).await?;
        let is_flatpak = !matches!(plan, MpvLaunchPlan::Direct(_));
        let (mut cmd, display_target) = build_mpv_command(
            &plan,
            &socket_path,
            initial_title,
            &self.config.extra_args,
        );

        let mut child = cmd.spawn().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                missing_mpv_error(is_flatpak)
            } else {
                VesselError::Player(format!("spawn mpv process ({display_target}): {err}"))
            }
        })?;

        let stderr_buf = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        if let Some(mut stderr_pipe) = child.stderr.take() {
            let stderr_buf_task = Arc::clone(&stderr_buf);
            tokio::spawn(async move {
                let mut chunk = [0u8; 1024];
                while let Ok(n) = stderr_pipe.read(&mut chunk).await {
                    if n == 0 {
                        break;
                    }
                    if let Ok(mut buf) = stderr_buf_task.lock() {
                        if buf.len() < MAX_STDERR_CAPTURE_BYTES {
                            let remaining = MAX_STDERR_CAPTURE_BYTES - buf.len();
                            let take = n.min(remaining);
                            buf.extend_from_slice(&chunk[..take]);
                        }
                    }
                }
            });
        }

        let deadline = tokio::time::Instant::now() + SOCKET_CONNECT_TIMEOUT;
        let ipc = loop {
            if let Ok(Some(status)) = child.try_wait() {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let stderr_msg = stderr_buf
                    .lock()
                    .ok()
                    .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
                    .unwrap_or_default();
                if stderr_msg.is_empty() {
                    return Err(VesselError::Player(format!(
                        "mpv exited early with status {status}"
                    )));
                }
                return Err(VesselError::Player(format!(
                    "mpv exited early with status {status}: {stderr_msg}"
                )));
            }
            match MpvIpcClient::connect(&socket_path, self.event_sink.clone()).await {
                Ok(client) => break client,
                Err(_) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(SOCKET_POLL_INTERVAL).await;
                }
                Err(err) => return Err(err),
            }
        };

        guard.child = Some(child);
        guard.ipc = Some(ipc);
        guard.socket_path = Some(socket_path);
        Ok(())
    }

    async fn send_command(&self, args: &[Value]) -> VesselResult<Value> {
        let guard = self.state.lock().await;
        let Some(ref ipc) = guard.ipc else {
            return Err(VesselError::Player("mpv IPC session is not active".into()));
        };
        ipc.command(args).await
    }

    async fn resolve_subtitle_target(&self, track: &SubtitleTrack) -> VesselResult<String> {
        match &track.source {
            SubtitleSource::Url { url, .. } => Ok(url.clone()),
            SubtitleSource::InlineContent { content } => {
                fs::create_dir_all(&self.config.runtime_dir).map_err(|err| {
                    VesselError::Player(format!("create mpv subtitle dir: {err}"))
                })?;
                let idx = self.sub_counter.fetch_add(1, Ordering::Relaxed);
                let ext = subtitle_extension(track.format);
                let file_path = self.config.runtime_dir.join(format!("sub-{idx}.{ext}"));
                fs::write(&file_path, content.as_bytes()).map_err(|err| {
                    VesselError::Player(format!("write temp subtitle file: {err}"))
                })?;
                let mut guard = self.state.lock().await;
                guard.temp_subtitles.push(file_path.clone());
                Ok(file_path.to_string_lossy().into_owned())
            }
        }
    }

    async fn clear_temp_subtitles(&self) {
        let mut guard = self.state.lock().await;
        for path in guard.temp_subtitles.drain(..) {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for MpvWindowBackend {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.state.try_lock() {
            for path in guard.temp_subtitles.drain(..) {
                let _ = fs::remove_file(path);
            }
            if let Some(path) = guard.socket_path.take() {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[async_trait]
impl PlayerBackend for MpvWindowBackend {
    async fn load(&self, request: PlaybackRequest) -> VesselResult<()> {
        self.clear_temp_subtitles().await;
        self.ensure_connected(request.title.as_deref()).await?;

        if let Some(title) = request.title.as_deref() {
            let _ = self
                .send_command(&[
                    Value::from("set_property"),
                    Value::from("force-media-title"),
                    Value::from(title),
                ])
                .await;
        }

        if !request.headers.is_empty() {
            let fields: Vec<String> = request
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            self.send_command(&[
                Value::from("set_property"),
                Value::from("http-header-fields"),
                Value::from(fields),
            ])
            .await?;
        }

        if request.start_position_ms > 0 {
            let start_secs = format!("start=+{:.3}", request.start_position_ms as f64 / 1000.0);
            self.send_command(&[
                Value::from("loadfile"),
                Value::from(request.url),
                Value::from("replace"),
                Value::from(-1),
                Value::from(start_secs),
            ])
            .await?;
        } else {
            self.send_command(&[
                Value::from("loadfile"),
                Value::from(request.url),
                Value::from("replace"),
            ])
            .await?;
        }

        for track in &request.subtitles {
            self.add_subtitle(track.clone()).await?;
        }

        Ok(())
    }

    async fn add_subtitle(&self, track: SubtitleTrack) -> VesselResult<()> {
        let target = self.resolve_subtitle_target(&track).await?;
        let flags = if track.is_default { "select" } else { "auto" };
        self.send_command(&[
            Value::from("sub-add"),
            Value::from(target),
            Value::from(flags),
            Value::from(track.label),
            Value::from(track.lang),
        ])
        .await?;
        Ok(())
    }

    async fn set_subtitle_delay(&self, delay_ms: i64) -> VesselResult<()> {
        let secs = delay_ms as f64 / 1000.0;
        self.send_command(&[
            Value::from("set_property"),
            Value::from("sub-delay"),
            Value::from(secs),
        ])
        .await?;
        Ok(())
    }

    async fn play(&self) -> VesselResult<()> {
        self.send_command(&[
            Value::from("set_property"),
            Value::from("pause"),
            Value::from(false),
        ])
        .await?;
        Ok(())
    }

    async fn pause(&self) -> VesselResult<()> {
        self.send_command(&[
            Value::from("set_property"),
            Value::from("pause"),
            Value::from(true),
        ])
        .await?;
        Ok(())
    }

    async fn seek(&self, position_ms: u64) -> VesselResult<()> {
        let secs = position_ms as f64 / 1000.0;
        self.send_command(&[
            Value::from("seek"),
            Value::from(secs),
            Value::from("absolute"),
        ])
        .await?;
        Ok(())
    }

    async fn stop(&self) -> VesselResult<()> {
        let res = self.send_command(&[Value::from("stop")]).await;
        self.clear_temp_subtitles().await;
        res?;
        Ok(())
    }

    async fn stats(&self) -> Option<PlaybackStats> {
        let read_f64 = |prop: &'static str| async move {
            self.send_command(&[Value::from("get_property"), Value::from(prop)])
                .await
                .ok()
                .and_then(|v| v.as_f64())
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        let read_str = |prop: &'static str| async move {
            self.send_command(&[Value::from("get_property"), Value::from(prop)])
                .await
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
        };

        let (buf, cache_spd, vbr, abr, w, h, fps, vc, ac, fsz) = tokio::join!(
            read_f64("demuxer-cache-duration"),
            read_f64("cache-speed"),
            read_f64("video-bitrate"),
            read_f64("audio-bitrate"),
            read_f64("video-params/w"),
            read_f64("video-params/h"),
            read_f64("estimated-vf-fps"),
            read_str("video-codec"),
            read_str("audio-codec"),
            read_f64("file-size"),
        );

        Some(PlaybackStats {
            network_speed_bps: cache_spd.map(|v| v as u64).unwrap_or(0),
            total_bytes_transferred: 0,
            buffer_seconds: buf,
            cache_speed_bps: cache_spd.map(|v| v as u64),
            video_bitrate_bps: vbr.map(|v| v as u64),
            audio_bitrate_bps: abr.map(|v| v as u64),
            width: w.map(|v| v as u32),
            height: h.map(|v| v as u32),
            fps,
            video_codec: vc,
            audio_codec: ac,
            file_size_bytes: fsz.map(|v| v as u64),
            is_debrid_stream: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use serde_json::Value;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;
    use vessel_domain::{
        EngineEvent, EventSink, MediaType, PlaybackRequest, PlayerBackend, SubtitleFormat,
        SubtitleSource, SubtitleTrack,
    };

    use super::{MpvConfig, MpvWindowBackend};

    #[derive(Default)]
    struct EventRecorder {
        events: Mutex<Vec<EngineEvent>>,
    }

    impl EventSink for EventRecorder {
        fn emit(&self, event: EngineEvent) {
            self.events.lock().expect("lock events").push(event);
        }
    }

    #[tokio::test]
    async fn sends_json_ipc_commands_and_emits_player_state_snapshots() {
        let runtime_dir = std::env::temp_dir().join(format!(
            "vessel-mpv-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&runtime_dir).expect("mkdir runtime dir");
        let sock_path = runtime_dir.join("mock-mpv.sock");

        let listener = UnixListener::bind(&sock_path).expect("bind mock mpv sock");
        let received = Arc::new(Mutex::new(Vec::<Value>::new()));
        let received_srv = Arc::clone(&received);

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept ipc client");
            let (read_half, mut write_half) = stream.into_split();
            let mut lines = BufReader::new(read_half).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let req: Value = serde_json::from_str(&line).expect("parse ipc req");
                let req_id = req["request_id"].as_u64().expect("req id");
                let cmd = req["command"].clone();
                received_srv
                    .lock()
                    .expect("lock received")
                    .push(cmd.clone());

                let reply = serde_json::json!({
                    "request_id": req_id,
                    "error": "success",
                    "data": null
                });
                write_half
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .expect("write ipc reply");

                if cmd[0] == "loadfile" {
                    for event_msg in [
                        serde_json::json!({ "event": "property-change", "name": "duration", "data": 1440.5 }),
                        serde_json::json!({ "event": "property-change", "name": "time-pos", "data": 12.25 }),
                    ] {
                        write_half
                            .write_all(format!("{event_msg}\n").as_bytes())
                            .await
                            .expect("write ipc event");
                    }
                }
            }
        });

        let recorder = Arc::new(EventRecorder::default());
        let backend = MpvWindowBackend::new(
            MpvConfig {
                runtime_dir: runtime_dir.clone(),
                ..MpvConfig::default()
            },
            Some(recorder.clone()),
        );
        backend
            .attach_ipc_socket(&sock_path)
            .await
            .expect("attach mock ipc");

        backend
            .load(PlaybackRequest {
                url: "http://127.0.0.1:42000/s/tok/master".into(),
                headers: BTreeMap::new(),
                media_type: MediaType::Series,
                start_position_ms: 5000,
                subtitles: vec![
                    SubtitleTrack {
                        id: Some("sub-url".into()),
                        source: SubtitleSource::Url {
                            url: "http://127.0.0.1:42000/s/sub/master".into(),
                            headers: BTreeMap::new(),
                        },
                        lang: "tr".into(),
                        label: "Turkish".into(),
                        format: SubtitleFormat::Ass,
                        is_default: true,
                    },
                    SubtitleTrack {
                        id: Some("sub-inline".into()),
                        source: SubtitleSource::InlineContent {
                            content: "1\n00:00:01,000 --> 00:00:02,000\nHello Vessel\n".into(),
                        },
                        lang: "en".into(),
                        label: "English Inline".into(),
                        format: SubtitleFormat::Srt,
                        is_default: false,
                    },
                ],
                title: Some("Anthology Ep 1".into()),
            })
            .await
            .expect("load media");

        backend
            .set_subtitle_delay(1250)
            .await
            .expect("set sub delay");
        backend.pause().await.expect("pause");
        backend.play().await.expect("play");
        backend.seek(30_000).await.expect("seek");

        // Give reader task a moment to flush property-change events before stop.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        backend.stop().await.expect("stop");

        {
            let cmds = received.lock().expect("lock cmds");
            assert!(
                cmds.iter()
                    .any(|c| c[0] == "loadfile" && c[4] == "start=+5.000")
            );
            assert!(
                cmds.iter()
                    .any(|c| c[0] == "sub-add" && c[1] == "http://127.0.0.1:42000/s/sub/master")
            );
            assert!(
                cmds.iter().any(
                    |c| c[0] == "sub-add" && c[1].as_str().is_some_and(|s| s.ends_with(".srt"))
                )
            );
            assert!(
                cmds.iter()
                    .any(|c| c[0] == "set_property" && c[1] == "sub-delay" && c[2] == 1.25)
            );
            assert!(
                cmds.iter()
                    .any(|c| c[0] == "seek" && c[1] == 30.0 && c[2] == "absolute")
            );
            assert!(cmds.iter().any(|c| c[0] == "stop"));
        }

        {
            let events = recorder.events.lock().expect("lock events");
            assert!(events.iter().any(|ev| matches!(
                ev,
                EngineEvent::PlayerStateChanged(snap) if snap.position_ms == 12250 && snap.duration_ms == 1440500
            )));
        }

        drop(backend);
        server_task.abort();
        let _ = std::fs::remove_dir_all(&runtime_dir);
    }

    #[test]
    fn resolve_default_runtime_dir_uses_shared_xdg_app_dir_inside_flatpak() {
        use std::path::Path;

        let dir = super::resolve_default_runtime_dir(
            true,
            Some(Path::new("/run/user/1000")),
            Some("dev.falsis.vessel"),
            Path::new("/tmp"),
            "vessel-mpv-1-1",
        );
        assert_eq!(
            dir,
            Path::new("/run/user/1000/app/dev.falsis.vessel/vessel-mpv-1-1")
        );

        let fallback = super::resolve_default_runtime_dir(
            false,
            Some(Path::new("/run/user/1000")),
            Some("dev.falsis.vessel"),
            Path::new("/tmp"),
            "vessel-mpv-1-1",
        );
        assert_eq!(fallback, Path::new("/tmp/vessel-mpv-1-1"));
    }

    #[test]
    fn parse_host_mpv_probe_output_handles_binary_and_flatpak_app() {
        assert_eq!(
            super::parse_host_mpv_probe_output("\n  /usr/bin/mpv \n"),
            Some(Ok("/usr/bin/mpv".to_owned()))
        );
        assert_eq!(
            super::parse_host_mpv_probe_output("__FLATPAK_APP__:io.mpv.Mpv\n"),
            Some(Err("io.mpv.Mpv".to_owned()))
        );
        assert_eq!(super::parse_host_mpv_probe_output("   \n "), None);
    }
}
