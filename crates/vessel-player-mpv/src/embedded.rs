//! Lazy, low-memory embedded `libmpv` engine with zero-copy GPU rendering and track state sync.

use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use vessel_domain::{VesselError, VesselResult};

use crate::libmpv_ffi::{
    LibMpv, MPV_END_FILE_REASON_EOF, MPV_END_FILE_REASON_ERROR, MPV_EVENT_END_FILE,
    MPV_EVENT_FILE_LOADED, MPV_EVENT_NONE, MPV_EVENT_PLAYBACK_RESTART, MPV_EVENT_PROPERTY_CHANGE,
    MPV_EVENT_SHUTDOWN, MPV_FORMAT_DOUBLE, MPV_FORMAT_FLAG, MPV_FORMAT_STRING, MpvHandle,
    MpvRenderContext, find_available_libmpv_path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddedFitMode {
    Contain,
    Cover,
    Fill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedTrackInfo {
    pub id: i64,
    pub track_type: String,
    pub title: Option<String>,
    pub lang: Option<String>,
    pub codec: Option<String>,
    pub channels: Option<String>,
    pub selected: bool,
    pub external: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedPlayerState {
    pub active: bool,
    pub is_playing: bool,
    pub is_paused: bool,
    pub is_buffering: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
    pub speed: f64,
    pub eof_reached: bool,
    pub audio_tracks: Vec<EmbeddedTrackInfo>,
    pub subtitle_tracks: Vec<EmbeddedTrackInfo>,
    pub error: Option<String>,
}

struct EmbeddedCore {
    lib: Arc<LibMpv>,
    handle: *mut MpvHandle,
    event_thread: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

unsafe impl Send for EmbeddedCore {}
unsafe impl Sync for EmbeddedCore {}

struct UpdateCallbackHolder {
    cb: Arc<dyn Fn() + Send + Sync>,
}

extern "C" fn on_render_update(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    let holder = unsafe { &*(ctx as *const UpdateCallbackHolder) };
    (holder.cb)();
}

extern "C" fn on_mpv_wakeup(_ctx: *mut c_void) {}

/// Thread-safe lazy embedded `libmpv` session controller.
pub struct MpvEmbeddedPlayer {
    core: Mutex<Option<EmbeddedCore>>,
    render_ctx: AtomicPtr<MpvRenderContext>,
    render_lib: Mutex<Option<Arc<LibMpv>>>,
    update_holder: Mutex<Option<Box<UpdateCallbackHolder>>>,
    state: Arc<Mutex<EmbeddedPlayerState>>,
    state_listener: Arc<Mutex<Option<Arc<dyn Fn(EmbeddedPlayerState) + Send + Sync>>>>,
    wid_target: Mutex<Option<i64>>,
}

unsafe impl Send for MpvEmbeddedPlayer {}
unsafe impl Sync for MpvEmbeddedPlayer {}

impl std::fmt::Debug for MpvEmbeddedPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MpvEmbeddedPlayer").finish_non_exhaustive()
    }
}

impl Default for MpvEmbeddedPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvEmbeddedPlayer {
    /// Creates an idle controller with zero allocated `libmpv` memory until playback starts.
    pub fn new() -> Self {
        Self {
            core: Mutex::new(None),
            render_ctx: AtomicPtr::new(std::ptr::null_mut()),
            render_lib: Mutex::new(None),
            update_holder: Mutex::new(None),
            state: Arc::new(Mutex::new(EmbeddedPlayerState {
                volume: 1.0,
                speed: 1.0,
                ..Default::default()
            })),
            state_listener: Arc::new(Mutex::new(None)),
            wid_target: Mutex::new(None),
        }
    }

    /// Probes whether `libmpv` is loadable on this machine.
    pub fn probe_availability() -> Option<String> {
        find_available_libmpv_path()
    }

    pub fn set_wid_target(&self, wid: Option<i64>) {
        if let Ok(mut guard) = self.wid_target.lock() {
            *guard = wid;
        }
    }

    pub fn set_state_listener(&self, listener: Arc<dyn Fn(EmbeddedPlayerState) + Send + Sync>) {
        if let Ok(mut guard) = self.state_listener.lock() {
            *guard = Some(listener);
        }
    }

    pub fn set_render_update_callback(&self, cb: Arc<dyn Fn() + Send + Sync>) {
        let raw_ptr = {
            let mut holder_guard = self.update_holder.lock().expect("lock update holder");
            let boxed = Box::new(UpdateCallbackHolder { cb });
            let ptr = Box::as_ref(&boxed) as *const UpdateCallbackHolder as *mut c_void;
            *holder_guard = Some(boxed);
            ptr
        };
        let rctx = self.render_ctx.load(Ordering::Acquire);
        if !rctx.is_null()
            && let Ok(lib_guard) = self.render_lib.lock()
            && let Some(lib) = lib_guard.as_ref()
        {
            unsafe {
                lib.render_context_set_update_callback(rctx, Some(on_render_update), raw_ptr);
            }
        }
    }

    /// Lazily initializes `libmpv` with low-memory streaming & hardware decoding options.
    pub fn ensure_initialized(&self) -> VesselResult<()> {
        let mut core_guard = self.core.lock().expect("lock embedded core");
        if core_guard.is_some() {
            return Ok(());
        }

        let lib = LibMpv::load()?;
        let handle = lib.create()?;

        let wid = *self.wid_target.lock().expect("lock wid");
        let setup_res = (|| -> VesselResult<()> {
            if let Some(win_id) = wid {
                lib.set_option_string(handle, "wid", &win_id.to_string())?;
                lib.set_option_string(handle, "vo", "gpu")?;
            } else {
                lib.set_option_string(handle, "vo", "libmpv")?;
            }
            lib.set_option_string(handle, "hwdec", "auto-safe")?;
            lib.set_option_string(handle, "keep-open", "yes")?;
            lib.set_option_string(handle, "idle", "yes")?;
            lib.set_option_string(handle, "cache", "yes")?;
            lib.set_option_string(handle, "demuxer-max-bytes", "67108864")?;
            lib.set_option_string(handle, "demuxer-max-back-bytes", "16710886")?;
            lib.set_option_string(handle, "input-default-bindings", "no")?;
            lib.set_option_string(handle, "input-vo-keyboard", "no")?;
            lib.set_option_string(handle, "osc", "no")?;
            lib.set_option_string(handle, "sub-visibility", "no")?;
            lib.set_option_string(handle, "video-timing-offset", "0")?;
            lib.initialize(handle)?;

            lib.observe_property(handle, 1, "time-pos", MPV_FORMAT_DOUBLE)?;
            lib.observe_property(handle, 2, "duration", MPV_FORMAT_DOUBLE)?;
            lib.observe_property(handle, 3, "pause", MPV_FORMAT_FLAG)?;
            lib.observe_property(handle, 4, "paused-for-cache", MPV_FORMAT_FLAG)?;
            lib.observe_property(handle, 5, "volume", MPV_FORMAT_DOUBLE)?;
            lib.observe_property(handle, 6, "speed", MPV_FORMAT_DOUBLE)?;
            lib.observe_property(handle, 7, "eof-reached", MPV_FORMAT_FLAG)?;
            lib.observe_property(handle, 8, "track-list", MPV_FORMAT_STRING)?;
            Ok(())
        })();

        if let Err(err) = setup_res {
            lib.terminate_destroy(handle);
            return Err(err);
        }

        lib.set_wakeup_callback(handle, Some(on_mpv_wakeup), std::ptr::null_mut());

        let running = Arc::new(AtomicBool::new(true));
        let running_ev = Arc::clone(&running);
        let lib_ev = Arc::clone(&lib);
        let state_ev = Arc::clone(&self.state);
        let listener_ev = Arc::clone(&self.state_listener);
        let handle_addr = handle as usize;

        let event_thread = std::thread::Builder::new()
            .name("vessel-libmpv-events".into())
            .spawn(move || {
                let handle_ptr = handle_addr as *mut MpvHandle;
                while running_ev.load(Ordering::Acquire) {
                    let ev_ptr = lib_ev.wait_event(handle_ptr, 0.25);
                    if ev_ptr.is_null() {
                        continue;
                    }
                    let ev = unsafe { &*ev_ptr };
                    if ev.event_id == MPV_EVENT_NONE {
                        continue;
                    }
                    if ev.event_id == MPV_EVENT_SHUTDOWN {
                        break;
                    }
                    let mut changed = false;
                    {
                        let mut st = state_ev.lock().expect("lock embedded state");
                        match ev.event_id {
                            MPV_EVENT_FILE_LOADED => {
                                st.active = true;
                                st.is_buffering = false;
                                st.error = None;
                                if let Some(raw_tracks) =
                                    lib_ev.get_property_string(handle_ptr, "track-list")
                                {
                                    apply_track_list(&raw_tracks, &mut st);
                                }
                                changed = true;
                            }
                            MPV_EVENT_PLAYBACK_RESTART => {
                                st.is_buffering = false;
                                st.is_playing = !st.is_paused;
                                changed = true;
                            }
                            MPV_EVENT_END_FILE => {
                                if !ev.data.is_null() {
                                    let ef = unsafe { &*(ev.data as *const crate::libmpv_ffi::MpvEventEndFile) };
                                    if ef.reason == MPV_END_FILE_REASON_ERROR {
                                        st.error = Some(lib_ev.error_string(ef.error));
                                        st.is_playing = false;
                                        st.is_buffering = false;
                                        changed = true;
                                    } else if ef.reason == MPV_END_FILE_REASON_EOF {
                                        st.eof_reached = true;
                                        st.is_playing = false;
                                        st.is_buffering = false;
                                        changed = true;
                                    }
                                }
                            }
                            MPV_EVENT_PROPERTY_CHANGE => {
                                if !ev.data.is_null() {
                                    let prop = unsafe {
                                        &*(ev.data as *const crate::libmpv_ffi::MpvEventProperty)
                                    };
                                    changed = apply_property_change(&lib_ev, prop, &mut st);
                                }
                            }
                            _ => {}
                        }
                    }
                    if changed {
                        let snap = state_ev.lock().expect("lock state").clone();
                        if let Some(listener) = listener_ev.lock().expect("lock listener").as_ref() {
                            listener(snap);
                        }
                    }
                }
            })
            .map_err(|err| VesselError::Player(format!("spawn libmpv event thread: {err}")))?;

        *self.render_lib.lock().expect("lock render lib") = Some(Arc::clone(&lib));
        *core_guard = Some(EmbeddedCore {
            lib,
            handle,
            event_thread: Some(event_thread),
            running,
        });
        Ok(())
    }

    /// Initializes the OpenGL render context on the caller's active GL thread (macOS/Linux).
    ///
    /// # Safety
    /// Must be invoked with a valid, current OpenGL/CGL/EGL context bound on the calling thread.
    pub unsafe fn ensure_opengl_render_context(
        &self,
        get_proc_address: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char) -> *mut c_void,
        get_proc_address_ctx: *mut c_void,
    ) -> VesselResult<()> {
        if !self.render_ctx.load(Ordering::Acquire).is_null() {
            return Ok(());
        }
        let core_guard = self.core.lock().expect("lock core");
        let Some(core) = core_guard.as_ref() else {
            return Err(VesselError::Player("libmpv core not initialized".into()));
        };
        let rctx = unsafe {
            core.lib
                .create_opengl_render_context(core.handle, get_proc_address, get_proc_address_ctx)?
        };
        if let Some(holder) = self.update_holder.lock().expect("lock holder").as_ref() {
            let ptr = holder.as_ref() as *const UpdateCallbackHolder as *mut c_void;
            unsafe {
                core.lib
                    .render_context_set_update_callback(rctx, Some(on_render_update), ptr);
            }
        }
        self.render_ctx.store(rctx, Ordering::Release);
        Ok(())
    }

    /// Renders the latest video frame into the caller's bound OpenGL FBO.
    pub fn render_gl_fbo(&self, fbo: i32, width: i32, height: i32, flip_y: bool) {
        let rctx = self.render_ctx.load(Ordering::Acquire);
        if rctx.is_null() {
            return;
        }
        let Ok(lib_guard) = self.render_lib.try_lock() else {
            return;
        };
        let Some(lib) = lib_guard.as_ref() else {
            return;
        };
        unsafe {
            let _ = lib.render_context_update(rctx);
            let _ = lib.render_context_render_fbo(rctx, fbo, width, height, flip_y);
            lib.render_context_report_swap(rctx);
        }
    }

    /// Frees the OpenGL render context (must run on the GL thread before `destroy`).
    pub fn free_opengl_render_context(&self) {
        let rctx = self.render_ctx.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if rctx.is_null() {
            return;
        }
        if let Ok(lib_guard) = self.render_lib.lock()
            && let Some(lib) = lib_guard.as_ref()
        {
            unsafe {
                lib.render_context_set_update_callback(rctx, None, std::ptr::null_mut());
                lib.render_context_free(rctx);
            }
        }
    }

    pub fn load(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        start_position_ms: u64,
    ) -> VesselResult<()> {
        self.ensure_initialized()?;

        let (lib, handle) = {
            let guard = self.core.lock().expect("lock core");
            let core = guard
                .as_ref()
                .ok_or_else(|| VesselError::Player("embedded libmpv core unavailable".into()))?;
            (Arc::clone(&core.lib), core.handle)
        };

        {
            let mut st = self.state.lock().expect("lock state");
            st.active = true;
            st.is_buffering = true;
            st.is_playing = false;
            st.is_paused = false;
            st.eof_reached = false;
            st.error = None;
            st.position_ms = start_position_ms;
            st.audio_tracks.clear();
            st.subtitle_tracks.clear();
        }
        self.emit_current_state();

        if !headers.is_empty() {
            let joined = headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(",");
            let _ = lib.set_property_string(handle, "http-header-fields", &joined);
        } else {
            let _ = lib.set_property_string(handle, "http-header-fields", "");
        }

        let _ = lib.set_property_flag(handle, "pause", false);
        if start_position_ms > 0 {
            let start_opt = format!("start=+{:.3}", start_position_ms as f64 / 1000.0);
            lib.command_async(handle, 1, &["loadfile", url, "replace", "-1", &start_opt])?;
        } else {
            lib.command_async(handle, 1, &["loadfile", url, "replace"])?;
        }
        Ok(())
    }

    pub fn play(&self) -> VesselResult<()> {
        self.with_core(|lib, handle| lib.set_property_flag(handle, "pause", false))
    }

    pub fn pause(&self) -> VesselResult<()> {
        self.with_core(|lib, handle| lib.set_property_flag(handle, "pause", true))
    }

    pub fn seek(&self, position_ms: u64) -> VesselResult<()> {
        let secs = format!("{:.3}", position_ms as f64 / 1000.0);
        self.with_core(|lib, handle| lib.command_async(handle, 2, &["seek", &secs, "absolute"]))
    }

    pub fn set_volume(&self, volume_0_to_1: f64) -> VesselResult<()> {
        let mpv_vol = (volume_0_to_1.clamp(0.0, 1.5)) * 100.0;
        self.with_core(|lib, handle| lib.set_property_f64(handle, "volume", mpv_vol))
    }

    pub fn set_speed(&self, speed: f64) -> VesselResult<()> {
        let clamped = speed.clamp(0.25, 4.0);
        self.with_core(|lib, handle| lib.set_property_f64(handle, "speed", clamped))
    }

    pub fn set_fit_mode(&self, mode: EmbeddedFitMode) -> VesselResult<()> {
        self.with_core(|lib, handle| match mode {
            EmbeddedFitMode::Contain => {
                lib.set_property_f64(handle, "panscan", 0.0)?;
                lib.set_property_string(handle, "keepaspect", "yes")
            }
            EmbeddedFitMode::Cover => {
                lib.set_property_string(handle, "keepaspect", "yes")?;
                lib.set_property_f64(handle, "panscan", 1.0)
            }
            EmbeddedFitMode::Fill => {
                lib.set_property_f64(handle, "panscan", 0.0)?;
                lib.set_property_string(handle, "keepaspect", "no")
            }
        })
    }

    pub fn select_audio_track(&self, track_id: i64) -> VesselResult<()> {
        self.with_core(|lib, handle| lib.set_property_i64(handle, "aid", track_id))
    }

    pub fn select_subtitle_track(&self, track_id: Option<i64>) -> VesselResult<()> {
        self.with_core(|lib, handle| match track_id {
            Some(id) if id > 0 => {
                lib.set_property_i64(handle, "sid", id)?;
                lib.set_property_flag(handle, "sub-visibility", true)
            }
            _ => {
                lib.set_property_flag(handle, "sub-visibility", false)?;
                lib.set_property_string(handle, "sid", "no")
            }
        })
    }

    pub fn stop(&self) -> VesselResult<()> {
        let res = self.with_core(|lib, handle| lib.command_async(handle, 3, &["stop"]));
        if let Ok(mut st) = self.state.lock() {
            st.active = false;
            st.is_playing = false;
            st.is_buffering = false;
        }
        self.emit_current_state();
        res
    }

    /// Stops playback and tears down `libmpv` completely so zero playback RAM remains allocated.
    pub fn destroy(&self) {
        self.free_opengl_render_context();
        let popped = {
            let mut guard = self.core.lock().expect("lock core");
            guard.take()
        };
        if let Some(mut core) = popped {
            core.running.store(false, Ordering::Release);
            let _ = core.lib.command_async(core.handle, 99, &["quit"]);
            if let Some(handle) = core.event_thread.take() {
                let _ = handle.join();
            }
            core.lib.terminate_destroy(core.handle);
        }
        *self.render_lib.lock().expect("lock render lib") = None;
        if let Ok(mut st) = self.state.lock() {
            *st = EmbeddedPlayerState {
                volume: st.volume,
                speed: st.speed,
                ..Default::default()
            };
        }
        self.emit_current_state();
    }

    fn with_core<F>(&self, f: F) -> VesselResult<()>
    where
        F: FnOnce(&LibMpv, *mut MpvHandle) -> VesselResult<()>,
    {
        let guard = self.core.lock().expect("lock embedded core");
        let Some(core) = guard.as_ref() else {
            return Err(VesselError::Player("embedded libmpv is not active".into()));
        };
        f(&core.lib, core.handle)
    }

    fn emit_current_state(&self) {
        let snap = self.state.lock().expect("lock state").clone();
        if let Some(listener) = self.state_listener.lock().expect("lock listener").as_ref() {
            listener(snap);
        }
    }
}

impl Drop for MpvEmbeddedPlayer {
    fn drop(&mut self) {
        self.destroy();
    }
}

fn secs_to_ms(secs: f64) -> u64 {
    if secs.is_finite() && secs >= 0.0 {
        (secs * 1000.0).round() as u64
    } else {
        0
    }
}

fn apply_property_change(
    _lib: &LibMpv,
    prop: &crate::libmpv_ffi::MpvEventProperty,
    st: &mut EmbeddedPlayerState,
) -> bool {
    if prop.name.is_null() || prop.data.is_null() {
        return false;
    }
    let name = unsafe { std::ffi::CStr::from_ptr(prop.name) }.to_string_lossy();
    match name.as_ref() {
        "time-pos" if prop.format == MPV_FORMAT_DOUBLE => {
            let secs = unsafe { *(prop.data as *const f64) };
            st.position_ms = secs_to_ms(secs);
            st.is_playing = !st.is_paused && !st.eof_reached;
            true
        }
        "duration" if prop.format == MPV_FORMAT_DOUBLE => {
            let secs = unsafe { *(prop.data as *const f64) };
            st.duration_ms = secs_to_ms(secs);
            true
        }
        "pause" if prop.format == MPV_FORMAT_FLAG => {
            let flag = unsafe { *(prop.data as *const std::ffi::c_int) };
            st.is_paused = flag != 0;
            st.is_playing = !st.is_paused && st.active && !st.eof_reached;
            true
        }
        "paused-for-cache" if prop.format == MPV_FORMAT_FLAG => {
            let flag = unsafe { *(prop.data as *const std::ffi::c_int) };
            st.is_buffering = flag != 0;
            true
        }
        "volume" if prop.format == MPV_FORMAT_DOUBLE => {
            let vol = unsafe { *(prop.data as *const f64) };
            st.volume = (vol / 100.0).clamp(0.0, 1.5);
            true
        }
        "speed" if prop.format == MPV_FORMAT_DOUBLE => {
            st.speed = unsafe { *(prop.data as *const f64) };
            true
        }
        "eof-reached" if prop.format == MPV_FORMAT_FLAG => {
            let flag = unsafe { *(prop.data as *const std::ffi::c_int) };
            st.eof_reached = flag != 0;
            if st.eof_reached {
                st.is_playing = false;
                st.is_buffering = false;
            }
            true
        }
        "track-list" if prop.format == MPV_FORMAT_STRING => {
            let raw_ptr = unsafe { *(prop.data as *const *const c_char) };
            if !raw_ptr.is_null() {
                let raw = unsafe { std::ffi::CStr::from_ptr(raw_ptr) }.to_string_lossy();
                apply_track_list(&raw, st);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

#[derive(Deserialize)]
struct RawMpvTrack {
    id: Option<i64>,
    #[serde(rename = "type")]
    track_type: Option<String>,
    title: Option<String>,
    lang: Option<String>,
    codec: Option<String>,
    #[serde(rename = "demux-channels")]
    demux_channels: Option<String>,
    #[serde(rename = "demux-channel-count")]
    demux_channel_count: Option<i64>,
    selected: Option<bool>,
    external: Option<bool>,
}

fn apply_track_list(raw_json: &str, st: &mut EmbeddedPlayerState) {
    let Ok(items) = serde_json::from_str::<Vec<RawMpvTrack>>(raw_json) else {
        return;
    };
    let mut audio = Vec::new();
    let mut subs = Vec::new();
    for item in items {
        let Some(id) = item.id else { continue };
        let Some(t) = item.track_type else { continue };
        let channels = item
            .demux_channels
            .filter(|s| !s.is_empty())
            .or_else(|| item.demux_channel_count.map(|c| format!("{c}ch")));
        let info = EmbeddedTrackInfo {
            id,
            track_type: t.clone(),
            title: item.title.filter(|s| !s.trim().is_empty()),
            lang: item.lang.filter(|s| !s.trim().is_empty() && s != "und"),
            codec: item.codec.filter(|s| !s.trim().is_empty()),
            channels,
            selected: item.selected.unwrap_or(false),
            external: item.external.unwrap_or(false),
        };
        if t == "audio" {
            audio.push(info);
        } else if t == "sub" {
            subs.push(info);
        }
    }
    st.audio_tracks = audio;
    st.subtitle_tracks = subs;
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedPlayerState, apply_track_list};

    #[test]
    fn parses_mpv_track_list_json_into_audio_and_sub_tracks() {
        let raw = r#"[
            {"id":1,"type":"video","codec":"hevc","selected":true},
            {"id":1,"type":"audio","title":"English DDP 5.1","lang":"eng","codec":"eac3","demux-channel-count":6,"selected":true},
            {"id":2,"type":"audio","title":"Hindi","lang":"hin","codec":"aac","selected":false},
            {"id":1,"type":"sub","title":"Signs & Songs","lang":"eng","codec":"ass","selected":false,"external":false}
        ]"#;
        let mut st = EmbeddedPlayerState::default();
        apply_track_list(raw, &mut st);
        assert_eq!(st.audio_tracks.len(), 2);
        assert_eq!(st.audio_tracks[0].codec.as_deref(), Some("eac3"));
        assert_eq!(st.audio_tracks[0].channels.as_deref(), Some("6ch"));
        assert_eq!(st.subtitle_tracks.len(), 1);
        assert_eq!(st.subtitle_tracks[0].codec.as_deref(), Some("ass"));
    }
}
