//! Dynamic `libloading` FFI bindings for `libmpv` (`client.h`, `render.h`, `render_gl.h`).
//!
//! Loading `libmpv` dynamically at runtime ensures crates and unit tests always build cleanly
//! without requiring system `libmpv-dev` headers at compile time, while enabling zero-copy
//! embedded GPU playback whenever `libmpv` is bundled or installed on the host.

#![allow(non_camel_case_types, dead_code)]

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;

use libloading::{Library, Symbol};
use vessel_domain::{VesselError, VesselResult};

pub type MpvHandle = c_void;
pub type MpvRenderContext = c_void;

pub const MPV_FORMAT_NONE: c_int = 0;
pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_OSD_STRING: c_int = 2;
pub const MPV_FORMAT_FLAG: c_int = 3;
pub const MPV_FORMAT_INT64: c_int = 4;
pub const MPV_FORMAT_DOUBLE: c_int = 5;

pub const MPV_EVENT_NONE: c_int = 0;
pub const MPV_EVENT_SHUTDOWN: c_int = 1;
pub const MPV_EVENT_END_FILE: c_int = 7;
pub const MPV_EVENT_FILE_LOADED: c_int = 8;
pub const MPV_EVENT_VIDEO_RECONFIG: c_int = 17;
pub const MPV_EVENT_PLAYBACK_RESTART: c_int = 21;
pub const MPV_EVENT_PROPERTY_CHANGE: c_int = 22;

pub const MPV_END_FILE_REASON_EOF: c_int = 0;
pub const MPV_END_FILE_REASON_STOP: c_int = 2;
pub const MPV_END_FILE_REASON_QUIT: c_int = 3;
pub const MPV_END_FILE_REASON_ERROR: c_int = 4;

pub const MPV_RENDER_PARAM_INVALID: c_int = 0;
pub const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
pub const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
pub const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
pub const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
pub const MPV_RENDER_PARAM_WL_DISPLAY: c_int = 9;
pub const MPV_RENDER_PARAM_X11_DISPLAY: c_int = 8;

pub const MPV_RENDER_UPDATE_FRAME: u64 = 1 << 0;

pub const MPV_RENDER_API_TYPE_OPENGL: &[u8] = b"opengl\0";

#[repr(C)]
pub struct MpvEvent {
    pub event_id: c_int,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct MpvEventProperty {
    pub name: *const c_char,
    pub format: c_int,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct MpvEventEndFile {
    pub reason: c_int,
    pub error: c_int,
    pub playlist_entry_id: i64,
    pub playlist_insert_id: i64,
    pub playlist_insert_num_entries: c_int,
}

#[repr(C)]
pub struct MpvRenderParam {
    pub param_type: c_int,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct MpvOpenglInitParams {
    pub get_proc_address: Option<unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char) -> *mut c_void>,
    pub get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
pub struct MpvOpenglFbo {
    pub fbo: c_int,
    pub w: c_int,
    pub h: c_int,
    pub internal_format: c_int,
}

pub type MpvWakeupFn = unsafe extern "C" fn(d: *mut c_void);
pub type MpvRenderUpdateFn = unsafe extern "C" fn(cb_ctx: *mut c_void);

type FnMpvCreate = unsafe extern "C" fn() -> *mut MpvHandle;
type FnMpvInitialize = unsafe extern "C" fn(*mut MpvHandle) -> c_int;
type FnMpvTerminateDestroy = unsafe extern "C" fn(*mut MpvHandle);
type FnMpvSetOptionString = unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int;
type FnMpvCommand = unsafe extern "C" fn(*mut MpvHandle, *const *const c_char) -> c_int;
type FnMpvCommandAsync = unsafe extern "C" fn(*mut MpvHandle, u64, *const *const c_char) -> c_int;
type FnMpvSetProperty = unsafe extern "C" fn(*mut MpvHandle, *const c_char, c_int, *mut c_void) -> c_int;
type FnMpvSetPropertyString = unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int;
type FnMpvGetProperty = unsafe extern "C" fn(*mut MpvHandle, *const c_char, c_int, *mut c_void) -> c_int;
type FnMpvObserveProperty = unsafe extern "C" fn(*mut MpvHandle, u64, *const c_char, c_int) -> c_int;
type FnMpvSetWakeupCallback = unsafe extern "C" fn(*mut MpvHandle, Option<MpvWakeupFn>, *mut c_void);
type FnMpvWaitEvent = unsafe extern "C" fn(*mut MpvHandle, c_double) -> *mut MpvEvent;
type FnMpvFree = unsafe extern "C" fn(*mut c_void);
type FnMpvErrorString = unsafe extern "C" fn(c_int) -> *const c_char;

type FnMpvRenderContextCreate =
    unsafe extern "C" fn(*mut *mut MpvRenderContext, *mut MpvHandle, *mut MpvRenderParam) -> c_int;
type FnMpvRenderContextSetUpdateCallback =
    unsafe extern "C" fn(*mut MpvRenderContext, Option<MpvRenderUpdateFn>, *mut c_void);
type FnMpvRenderContextUpdate = unsafe extern "C" fn(*mut MpvRenderContext) -> u64;
type FnMpvRenderContextRender =
    unsafe extern "C" fn(*mut MpvRenderContext, *mut MpvRenderParam) -> c_int;
type FnMpvRenderContextReportSwap = unsafe extern "C" fn(*mut MpvRenderContext);
type FnMpvRenderContextFree = unsafe extern "C" fn(*mut MpvRenderContext);

/// Dynamically loaded `libmpv` symbol table kept alive for the lifetime of the library handle.
pub struct LibMpv {
    _lib: Library,
    pub loaded_path: String,
    mpv_create: FnMpvCreate,
    mpv_initialize: FnMpvInitialize,
    mpv_terminate_destroy: FnMpvTerminateDestroy,
    mpv_set_option_string: FnMpvSetOptionString,
    mpv_command: FnMpvCommand,
    mpv_command_async: FnMpvCommandAsync,
    mpv_set_property: FnMpvSetProperty,
    mpv_set_property_string: FnMpvSetPropertyString,
    mpv_get_property: FnMpvGetProperty,
    mpv_observe_property: FnMpvObserveProperty,
    mpv_set_wakeup_callback: FnMpvSetWakeupCallback,
    mpv_wait_event: FnMpvWaitEvent,
    mpv_free: FnMpvFree,
    mpv_error_string: FnMpvErrorString,
    mpv_render_context_create: FnMpvRenderContextCreate,
    mpv_render_context_set_update_callback: FnMpvRenderContextSetUpdateCallback,
    mpv_render_context_update: FnMpvRenderContextUpdate,
    mpv_render_context_render: FnMpvRenderContextRender,
    mpv_render_context_report_swap: FnMpvRenderContextReportSwap,
    mpv_render_context_free: FnMpvRenderContextFree,
}

unsafe impl Send for LibMpv {}
unsafe impl Sync for LibMpv {}

impl std::fmt::Debug for LibMpv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibMpv")
            .field("loaded_path", &self.loaded_path)
            .finish_non_exhaustive()
    }
}

/// Returns candidate filesystem paths and sonames to probe for `libmpv` on the current OS.
pub fn candidate_libmpv_paths() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        #[cfg(target_os = "macos")]
        {
            if let Some(contents) = exe_dir.parent() {
                out.push(contents.join("Frameworks/libmpv.2.dylib").to_string_lossy().into_owned());
                out.push(contents.join("Frameworks/libmpv.dylib").to_string_lossy().into_owned());
            }
            out.push(exe_dir.join("libmpv.2.dylib").to_string_lossy().into_owned());
        }
        #[cfg(target_os = "windows")]
        {
            out.push(exe_dir.join("libmpv-2.dll").to_string_lossy().into_owned());
            out.push(exe_dir.join("mpv-2.dll").to_string_lossy().into_owned());
        }
        #[cfg(target_os = "linux")]
        {
            out.push(exe_dir.join("libmpv.so.2").to_string_lossy().into_owned());
            out.push(exe_dir.join("lib/libmpv.so.2").to_string_lossy().into_owned());
        }
    }

    #[cfg(target_os = "macos")]
    {
        out.extend([
            "/opt/homebrew/opt/mpv/lib/libmpv.2.dylib".to_owned(),
            "/opt/homebrew/lib/libmpv.2.dylib".to_owned(),
            "/usr/local/opt/mpv/lib/libmpv.2.dylib".to_owned(),
            "/usr/local/lib/libmpv.2.dylib".to_owned(),
            "/Applications/mpv.app/Contents/MacOS/lib/libmpv.2.dylib".to_owned(),
            "libmpv.2.dylib".to_owned(),
            "libmpv.dylib".to_owned(),
        ]);
    }
    #[cfg(target_os = "linux")]
    {
        out.extend([
            "/app/lib/libmpv.so.2".to_owned(),
            "libmpv.so.2".to_owned(),
            "libmpv.so.1".to_owned(),
            "libmpv.so".to_owned(),
            "/usr/lib/x86_64-linux-gnu/libmpv.so.2".to_owned(),
            "/usr/lib/aarch64-linux-gnu/libmpv.so.2".to_owned(),
            "/usr/lib64/libmpv.so.2".to_owned(),
            "/usr/lib/libmpv.so.2".to_owned(),
        ]);
    }
    #[cfg(target_os = "windows")]
    {
        out.extend([
            "libmpv-2.dll".to_owned(),
            "mpv-2.dll".to_owned(),
        ]);
    }
    out
}

/// Probes whether `libmpv` can be dynamically loaded without holding a permanent instance.
pub fn find_available_libmpv_path() -> Option<String> {
    for candidate in candidate_libmpv_paths() {
        let path = Path::new(&candidate);
        if path.is_absolute() && !path.is_file() {
            continue;
        }
        if unsafe { Library::new(&candidate) }.is_ok() {
            return Some(candidate);
        }
    }
    None
}

unsafe fn load_sym<T: Copy>(lib: &Library, name: &[u8], path: &str) -> VesselResult<T> {
    unsafe {
        let sym: Symbol<'_, T> = lib.get(name).map_err(|err| {
            VesselError::Player(format!(
                "resolve symbol '{}' from {path}: {err}",
                String::from_utf8_lossy(name).trim_end_matches('\0')
            ))
        })?;
        Ok(*sym)
    }
}

impl LibMpv {
    /// Dynamically loads `libmpv` from the bundled path or host system locations.
    pub fn load() -> VesselResult<Arc<Self>> {
        let mut last_err = String::new();
        for candidate in candidate_libmpv_paths() {
            let p = PathBuf::from(&candidate);
            if p.is_absolute() && !p.is_file() {
                continue;
            }
            match unsafe { Library::new(&candidate) } {
                Ok(lib) => {
                    let loaded = unsafe {
                        Self {
                            mpv_create: load_sym(&lib, b"mpv_create\0", &candidate)?,
                            mpv_initialize: load_sym(&lib, b"mpv_initialize\0", &candidate)?,
                            mpv_terminate_destroy: load_sym(
                                &lib,
                                b"mpv_terminate_destroy\0",
                                &candidate,
                            )?,
                            mpv_set_option_string: load_sym(
                                &lib,
                                b"mpv_set_option_string\0",
                                &candidate,
                            )?,
                            mpv_command: load_sym(&lib, b"mpv_command\0", &candidate)?,
                            mpv_command_async: load_sym(&lib, b"mpv_command_async\0", &candidate)?,
                            mpv_set_property: load_sym(&lib, b"mpv_set_property\0", &candidate)?,
                            mpv_set_property_string: load_sym(
                                &lib,
                                b"mpv_set_property_string\0",
                                &candidate,
                            )?,
                            mpv_get_property: load_sym(&lib, b"mpv_get_property\0", &candidate)?,
                            mpv_observe_property: load_sym(
                                &lib,
                                b"mpv_observe_property\0",
                                &candidate,
                            )?,
                            mpv_set_wakeup_callback: load_sym(
                                &lib,
                                b"mpv_set_wakeup_callback\0",
                                &candidate,
                            )?,
                            mpv_wait_event: load_sym(&lib, b"mpv_wait_event\0", &candidate)?,
                            mpv_free: load_sym(&lib, b"mpv_free\0", &candidate)?,
                            mpv_error_string: load_sym(&lib, b"mpv_error_string\0", &candidate)?,
                            mpv_render_context_create: load_sym(
                                &lib,
                                b"mpv_render_context_create\0",
                                &candidate,
                            )?,
                            mpv_render_context_set_update_callback: load_sym(
                                &lib,
                                b"mpv_render_context_set_update_callback\0",
                                &candidate,
                            )?,
                            mpv_render_context_update: load_sym(
                                &lib,
                                b"mpv_render_context_update\0",
                                &candidate,
                            )?,
                            mpv_render_context_render: load_sym(
                                &lib,
                                b"mpv_render_context_render\0",
                                &candidate,
                            )?,
                            mpv_render_context_report_swap: load_sym(
                                &lib,
                                b"mpv_render_context_report_swap\0",
                                &candidate,
                            )?,
                            mpv_render_context_free: load_sym(
                                &lib,
                                b"mpv_render_context_free\0",
                                &candidate,
                            )?,
                            _lib: lib,
                            loaded_path: candidate,
                        }
                    };
                    return Ok(Arc::new(loaded));
                }
                Err(err) => {
                    last_err = format!("{candidate}: {err}");
                }
            }
        }
        Err(VesselError::Player(format!(
            "embedded libmpv library not found ({last_err})"
        )))
    }

    pub fn error_string(&self, code: c_int) -> String {
        unsafe {
            let ptr = (self.mpv_error_string)(code);
            if ptr.is_null() {
                format!("mpv error {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn check(&self, code: c_int, op: &str) -> VesselResult<()> {
        if code >= 0 {
            Ok(())
        } else {
            Err(VesselError::Player(format!(
                "{op}: {}",
                self.error_string(code)
            )))
        }
    }

    pub fn create(&self) -> VesselResult<*mut MpvHandle> {
        let handle = unsafe { (self.mpv_create)() };
        if handle.is_null() {
            Err(VesselError::Player("mpv_create returned null".into()))
        } else {
            Ok(handle)
        }
    }

    pub fn initialize(&self, handle: *mut MpvHandle) -> VesselResult<()> {
        let code = unsafe { (self.mpv_initialize)(handle) };
        self.check(code, "mpv_initialize")
    }

    pub fn terminate_destroy(&self, handle: *mut MpvHandle) {
        if !handle.is_null() {
            unsafe { (self.mpv_terminate_destroy)(handle) };
        }
    }

    pub fn set_option_string(&self, handle: *mut MpvHandle, name: &str, val: &str) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid option name".into()))?;
        let c_val = CString::new(val).map_err(|_| VesselError::Player("invalid option value".into()))?;
        let code = unsafe { (self.mpv_set_option_string)(handle, c_name.as_ptr(), c_val.as_ptr()) };
        self.check(code, name)
    }

    pub fn set_property_string(&self, handle: *mut MpvHandle, name: &str, val: &str) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid prop name".into()))?;
        let c_val = CString::new(val).map_err(|_| VesselError::Player("invalid prop value".into()))?;
        let code = unsafe { (self.mpv_set_property_string)(handle, c_name.as_ptr(), c_val.as_ptr()) };
        self.check(code, name)
    }

    pub fn set_property_flag(&self, handle: *mut MpvHandle, name: &str, val: bool) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid prop name".into()))?;
        let mut raw: c_int = if val { 1 } else { 0 };
        let code = unsafe {
            (self.mpv_set_property)(handle, c_name.as_ptr(), MPV_FORMAT_FLAG, (&raw as *const c_int).cast_mut().cast())
        };
        let _ = &mut raw;
        self.check(code, name)
    }

    pub fn set_property_i64(&self, handle: *mut MpvHandle, name: &str, val: i64) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid prop name".into()))?;
        let mut raw = val;
        let code = unsafe {
            (self.mpv_set_property)(handle, c_name.as_ptr(), MPV_FORMAT_INT64, (&raw as *const i64).cast_mut().cast())
        };
        let _ = &mut raw;
        self.check(code, name)
    }

    pub fn set_property_f64(&self, handle: *mut MpvHandle, name: &str, val: f64) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid prop name".into()))?;
        let mut raw = val;
        let code = unsafe {
            (self.mpv_set_property)(handle, c_name.as_ptr(), MPV_FORMAT_DOUBLE, (&raw as *const f64).cast_mut().cast())
        };
        let _ = &mut raw;
        self.check(code, name)
    }

    pub fn get_property_string(&self, handle: *mut MpvHandle, name: &str) -> Option<String> {
        let c_name = CString::new(name).ok()?;
        let mut out: *mut c_char = ptr::null_mut();
        let code = unsafe {
            (self.mpv_get_property)(
                handle,
                c_name.as_ptr(),
                MPV_FORMAT_STRING,
                (&mut out as *mut *mut c_char).cast(),
            )
        };
        if code < 0 || out.is_null() {
            return None;
        }
        let s = unsafe { CStr::from_ptr(out).to_string_lossy().into_owned() };
        unsafe { (self.mpv_free)(out.cast()) };
        Some(s)
    }

    pub fn command(&self, handle: *mut MpvHandle, args: &[&str]) -> VesselResult<()> {
        let c_args: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).map_err(|_| VesselError::Player("invalid command arg".into())))
            .collect::<VesselResult<_>>()?;
        let mut ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(ptr::null());
        let code = unsafe { (self.mpv_command)(handle, ptrs.as_ptr()) };
        self.check(code, args.first().copied().unwrap_or("mpv_command"))
    }

    pub fn command_async(&self, handle: *mut MpvHandle, userdata: u64, args: &[&str]) -> VesselResult<()> {
        let c_args: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).map_err(|_| VesselError::Player("invalid command arg".into())))
            .collect::<VesselResult<_>>()?;
        let mut ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(ptr::null());
        let code = unsafe { (self.mpv_command_async)(handle, userdata, ptrs.as_ptr()) };
        self.check(code, args.first().copied().unwrap_or("mpv_command_async"))
    }

    pub fn observe_property(
        &self,
        handle: *mut MpvHandle,
        userdata: u64,
        name: &str,
        format: c_int,
    ) -> VesselResult<()> {
        let c_name = CString::new(name).map_err(|_| VesselError::Player("invalid observe name".into()))?;
        let code = unsafe { (self.mpv_observe_property)(handle, userdata, c_name.as_ptr(), format) };
        self.check(code, name)
    }

    pub fn set_wakeup_callback(
        &self,
        handle: *mut MpvHandle,
        cb: Option<MpvWakeupFn>,
        ctx: *mut c_void,
    ) {
        unsafe { (self.mpv_set_wakeup_callback)(handle, cb, ctx) };
    }

    pub fn wait_event(&self, handle: *mut MpvHandle, timeout_secs: f64) -> *mut MpvEvent {
        unsafe { (self.mpv_wait_event)(handle, timeout_secs) }
    }

    pub unsafe fn create_opengl_render_context(
        &self,
        handle: *mut MpvHandle,
        get_proc_address: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char) -> *mut c_void,
        get_proc_address_ctx: *mut c_void,
    ) -> VesselResult<*mut MpvRenderContext> {
        let mut init_params = MpvOpenglInitParams {
            get_proc_address: Some(get_proc_address),
            get_proc_address_ctx,
        };
        let mut params = [
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_API_TYPE,
                data: MPV_RENDER_API_TYPE_OPENGL.as_ptr().cast_mut().cast(),
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: (&mut init_params as *mut MpvOpenglInitParams).cast(),
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];
        let mut ctx: *mut MpvRenderContext = ptr::null_mut();
        let code = unsafe { (self.mpv_render_context_create)(&mut ctx, handle, params.as_mut_ptr()) };
        self.check(code, "mpv_render_context_create")?;
        Ok(ctx)
    }

    pub unsafe fn render_context_set_update_callback(
        &self,
        ctx: *mut MpvRenderContext,
        cb: Option<MpvRenderUpdateFn>,
        cb_ctx: *mut c_void,
    ) {
        if !ctx.is_null() {
            unsafe { (self.mpv_render_context_set_update_callback)(ctx, cb, cb_ctx) };
        }
    }

    pub unsafe fn render_context_update(&self, ctx: *mut MpvRenderContext) -> u64 {
        if ctx.is_null() {
            0
        } else {
            unsafe { (self.mpv_render_context_update)(ctx) }
        }
    }

    pub unsafe fn render_context_render_fbo(
        &self,
        ctx: *mut MpvRenderContext,
        fbo: c_int,
        width: c_int,
        height: c_int,
        flip_y: bool,
    ) -> VesselResult<()> {
        let mut gl_fbo = MpvOpenglFbo {
            fbo,
            w: width.max(1),
            h: height.max(1),
            internal_format: 0,
        };
        let mut flip: c_int = if flip_y { 1 } else { 0 };
        let mut params = [
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_OPENGL_FBO,
                data: (&mut gl_fbo as *mut MpvOpenglFbo).cast(),
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_FLIP_Y,
                data: (&mut flip as *mut c_int).cast(),
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];
        let code = unsafe { (self.mpv_render_context_render)(ctx, params.as_mut_ptr()) };
        self.check(code, "mpv_render_context_render")
    }

    pub unsafe fn render_context_report_swap(&self, ctx: *mut MpvRenderContext) {
        if !ctx.is_null() {
            unsafe { (self.mpv_render_context_report_swap)(ctx) };
        }
    }

    pub unsafe fn render_context_free(&self, ctx: *mut MpvRenderContext) {
        if !ctx.is_null() {
            unsafe { (self.mpv_render_context_free)(ctx) };
        }
    }
}
