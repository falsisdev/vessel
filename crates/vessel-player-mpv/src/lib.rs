//! Desktop `mpv` window backend implementing `vessel_domain::PlayerBackend` via JSON IPC.

pub mod ipc;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde_json::Value;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use vessel_domain::{
    EventSink, PlaybackRequest, PlayerBackend, SubtitleFormat, SubtitleSource, SubtitleTrack,
    VesselError, VesselResult,
};

use ipc::MpvIpcClient;

const SOCKET_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(25);
static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

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
        Self {
            binary_path: PathBuf::from("mpv"),
            extra_args: Vec::new(),
            runtime_dir: std::env::temp_dir().join(format!("vessel-mpv-{nanos}-{seq}")),
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

        let resolved_bin = resolve_mpv_binary(&self.config.binary_path);
        let mut cmd = Command::new(&resolved_bin);
        cmd.arg("--idle=yes")
            .arg("--force-window=yes")
            .arg("--no-terminal")
            .arg(format!("--input-ipc-server={}", socket_path.display()));
        if let Some(title) = initial_title {
            cmd.arg(format!("--title={title}"));
        }
        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                VesselError::Player(
                    "mpv executable not found. Install mpv (e.g. 'brew install mpv' on macOS or winget/scoop on Windows) to launch external playback."
                        .into(),
                )
            } else {
                VesselError::Player(format!(
                    "spawn mpv process ({}): {err}",
                    resolved_bin.display()
                ))
            }
        })?;

        let deadline = tokio::time::Instant::now() + SOCKET_CONNECT_TIMEOUT;
        let ipc = loop {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(VesselError::Player(format!(
                    "mpv exited early with status {status}"
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
}
