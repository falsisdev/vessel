//! Bridges `VesselEngine`'s `EventSink` trait onto Tauri's `vessel://event` channel.

use tauri::{AppHandle, Emitter};
use vessel_domain::{EngineEvent, EventSink};

/// Canonical Tauri event name carrying serialized `EngineEvent` payloads.
pub const VESSEL_EVENT_CHANNEL: &str = "vessel://event";

/// `EventSink` implementation forwarding all core events to the Vue frontend.
#[derive(Clone)]
pub struct TauriEventSink {
    app: AppHandle,
}

impl std::fmt::Debug for TauriEventSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TauriEventSink").finish_non_exhaustive()
    }
}

impl TauriEventSink {
    /// Wraps `AppHandle` to stream `EngineEvent` payloads into Tauri webviews.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: EngineEvent) {
        let _ = self.app.emit(VESSEL_EVENT_CHANNEL, event);
    }
}
