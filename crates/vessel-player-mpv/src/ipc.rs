//! Line-delimited JSON IPC client for `mpv` (`--input-ipc-server`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{Mutex, oneshot};
use vessel_domain::{EngineEvent, EventSink, PlayerStateSnapshot, VesselError, VesselResult};

const IPC_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
struct IpcRequest<'a> {
    command: &'a [Value],
    request_id: u64,
}

type PendingMap = Arc<StdMutex<HashMap<u64, oneshot::Sender<VesselResult<Value>>>>>;

/// Asynchronous JSON IPC connection to a running `mpv` instance.
pub struct MpvIpcClient {
    writer: Mutex<tokio::net::unix::OwnedWriteHalf>,
    next_id: AtomicU64,
    pending: PendingMap,
    reader_task: tokio::task::JoinHandle<()>,
}

impl std::fmt::Debug for MpvIpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MpvIpcClient")
            .field("next_id", &self.next_id.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl MpvIpcClient {
    /// Connects to an `mpv` Unix domain IPC socket and begins observing playback state.
    pub async fn connect(
        socket_path: &Path,
        event_sink: Option<Arc<dyn EventSink>>,
    ) -> VesselResult<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|err| VesselError::Player(format!("connect mpv IPC socket: {err}")))?;
        let (read_half, write_half) = stream.into_split();

        let pending: PendingMap = Arc::new(StdMutex::new(HashMap::new()));
        let pending_reader = Arc::clone(&pending);
        let state = Arc::new(StdMutex::new(PlayerStateSnapshot {
            is_playing: false,
            is_paused: false,
            is_buffering: false,
            position_ms: 0,
            duration_ms: 0,
            current_stream_url: None,
            subtitle_delay_ms: 0,
        }));

        let reader_task = tokio::spawn(async move {
            let mut lines = BufReader::new(read_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };

                if let Some(req_id) = msg.get("request_id").and_then(Value::as_u64) {
                    let tx = pending_reader
                        .lock()
                        .ok()
                        .and_then(|mut map| map.remove(&req_id));
                    if let Some(sender) = tx {
                        let status = msg
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        let res = if status == "success" {
                            Ok(msg.get("data").cloned().unwrap_or(Value::Null))
                        } else {
                            Err(VesselError::Player(format!("mpv IPC error: {status}")))
                        };
                        let _ = sender.send(res);
                    }
                    continue;
                }

                if let Some(sink) = event_sink.as_ref()
                    && let Some(snapshot) = handle_ipc_event(&msg, &state)
                {
                    sink.emit(EngineEvent::PlayerStateChanged(snapshot));
                }
            }
        });

        let client = Self {
            writer: Mutex::new(write_half),
            next_id: AtomicU64::new(1),
            pending,
            reader_task,
        };

        for (id, prop) in [
            (1u64, "time-pos"),
            (2u64, "duration"),
            (3u64, "pause"),
            (4u64, "core-idle"),
            (5u64, "sub-delay"),
            (6u64, "path"),
        ] {
            client
                .command(&[
                    Value::from("observe_property"),
                    Value::from(id),
                    Value::from(prop),
                ])
                .await?;
        }

        Ok(client)
    }

    /// Sends a JSON IPC command and awaits `mpv`'s `request_id` response.
    pub async fn command(&self, args: &[Value]) -> VesselResult<Value> {
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let payload = serde_json::to_string(&IpcRequest {
            command: args,
            request_id,
        })
        .map_err(|err| VesselError::Player(format!("serialize mpv IPC command: {err}")))?;

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self
                .pending
                .lock()
                .map_err(|err| VesselError::Player(format!("ipc pending lock: {err}")))?;
            map.insert(request_id, tx);
        }

        {
            let mut writer = self.writer.lock().await;
            if let Err(err) = writer.write_all(format!("{payload}\n").as_bytes()).await {
                if let Ok(mut map) = self.pending.lock() {
                    map.remove(&request_id);
                }
                return Err(VesselError::Player(format!("write mpv IPC socket: {err}")));
            }
            if let Err(err) = writer.flush().await {
                if let Ok(mut map) = self.pending.lock() {
                    map.remove(&request_id);
                }
                return Err(VesselError::Player(format!("flush mpv IPC socket: {err}")));
            }
        }

        match tokio::time::timeout(IPC_COMMAND_TIMEOUT, rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err(VesselError::Player("mpv IPC channel closed".into())),
            Err(_) => {
                if let Ok(mut map) = self.pending.lock() {
                    map.remove(&request_id);
                }
                Err(VesselError::Timeout)
            }
        }
    }
}

impl Drop for MpvIpcClient {
    fn drop(&mut self) {
        self.reader_task.abort();
    }
}

fn secs_to_ms(secs: f64) -> u64 {
    if secs.is_finite() && secs >= 0.0 {
        (secs * 1000.0).round() as u64
    } else {
        0
    }
}

fn signed_secs_to_ms(secs: f64) -> i64 {
    if secs.is_finite() {
        (secs * 1000.0).round() as i64
    } else {
        0
    }
}

fn handle_ipc_event(
    msg: &Value,
    state: &StdMutex<PlayerStateSnapshot>,
) -> Option<PlayerStateSnapshot> {
    let event = msg.get("event")?.as_str()?;
    let mut guard = state.lock().ok()?;

    match event {
        "property-change" => {
            let name = msg.get("name")?.as_str()?;
            let data = msg.get("data").unwrap_or(&Value::Null);
            match name {
                "time-pos" => {
                    if let Some(secs) = data.as_f64() {
                        guard.position_ms = secs_to_ms(secs);
                        guard.is_playing = !guard.is_paused;
                    }
                }
                "duration" => {
                    if let Some(secs) = data.as_f64() {
                        guard.duration_ms = secs_to_ms(secs);
                    }
                }
                "pause" => {
                    if let Some(paused) = data.as_bool() {
                        guard.is_paused = paused;
                        guard.is_playing = !paused;
                    }
                }
                "core-idle" => {
                    if let Some(idle) = data.as_bool() {
                        guard.is_buffering = idle && !guard.is_paused;
                    }
                }
                "sub-delay" => {
                    if let Some(secs) = data.as_f64() {
                        guard.subtitle_delay_ms = signed_secs_to_ms(secs);
                    }
                }
                "path" => {
                    guard.current_stream_url = data.as_str().map(str::to_owned);
                }
                _ => return None,
            }
            Some(guard.clone())
        }
        "end-file" => {
            guard.is_playing = false;
            guard.is_buffering = false;
            Some(guard.clone())
        }
        _ => None,
    }
}
