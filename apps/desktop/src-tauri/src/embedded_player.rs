//! Native platform video surface controller embedding `libmpv` behind Tauri's transparent WebView.

use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::sync::OnceLock;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicUsize, Ordering};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use vessel_domain::{VesselError, VesselResult};
use vessel_player_mpv::embedded::{EmbeddedPlayerState, MpvEmbeddedPlayer};

pub const EMBEDDED_PLAYER_EVENT: &str = "vessel://embedded-player-state";

#[cfg(target_os = "macos")]
static SHARED_PLAYER: OnceLock<Arc<MpvEmbeddedPlayer>> = OnceLock::new();
#[cfg(target_os = "macos")]
static MACOS_GL_VIEW: AtomicUsize = AtomicUsize::new(0);

/// Holds the shared lazy `MpvEmbeddedPlayer` and binds it to the main Tauri window surface.
pub struct EmbeddedPlayerHost {
    pub player: Arc<MpvEmbeddedPlayer>,
}

impl EmbeddedPlayerHost {
    pub fn new(app: AppHandle) -> Self {
        let player = Arc::new(MpvEmbeddedPlayer::new());
        let app_for_events = app.clone();
        player.set_state_listener(Arc::new(move |state: EmbeddedPlayerState| {
            let _ = app_for_events.emit(EMBEDDED_PLAYER_EVENT, state);
        }));
        #[cfg(target_os = "macos")]
        {
            let _ = SHARED_PLAYER.set(Arc::clone(&player));
        }
        Self { player }
    }

    pub fn set_surface_visible(&self, app: &AppHandle, visible: bool) -> VesselResult<()> {
        let Some(window) = app.get_webview_window("main") else {
            return Err(VesselError::Player("main window unavailable".into()));
        };
        #[cfg(target_os = "macos")]
        {
            macos_set_surface_visible(&window, visible)?;
        }
        #[cfg(target_os = "linux")]
        {
            linux_set_surface_visible(&window, &self.player, visible)?;
        }
        #[cfg(target_os = "windows")]
        {
            windows_set_surface_visible(&window, &self.player, visible)?;
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = (window, visible);
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn macos_set_surface_visible(window: &WebviewWindow, visible: bool) -> VesselResult<()> {
    use std::ffi::{c_char, c_int, c_void};

    use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
    use objc2::{Encode, Encoding, class, msg_send, sel};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding =
            Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding =
            Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding =
            Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    #[link(name = "OpenGL", kind = "framework")]
    unsafe extern "C" {
        fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
        fn glClear(mask: u32);
    }

    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const RTLD_DEFAULT: *mut c_void = (-2isize) as *mut c_void;
    const NS_WINDOW_BELOW: isize = -1;
    const NS_VIEW_WIDTH_SIZABLE: usize = 2;
    const NS_VIEW_HEIGHT_SIZABLE: usize = 16;

    unsafe extern "C" fn gl_get_proc_address(
        _ctx: *mut c_void,
        name: *const c_char,
    ) -> *mut c_void {
        if name.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { dlsym(RTLD_DEFAULT, name) }
    }

    extern "C" fn trigger_view_redraw(ctx: *mut c_void) {
        if ctx.is_null() {
            return;
        }
        unsafe {
            let view = ctx as *mut AnyObject;
            let _: () = msg_send![view, setNeedsDisplay: Bool::YES];
        }
    }

    unsafe extern "C" fn prepare_opengl(this: *mut AnyObject, _sel: Sel) {
        let super_cls = AnyClass::get(c"NSOpenGLView").expect("NSOpenGLView class");
        let _: () = unsafe { msg_send![super(this, super_cls), prepareOpenGL] };
        let ctx: *mut AnyObject = unsafe { msg_send![this, openGLContext] };
        if !ctx.is_null() {
            let _: () = unsafe { msg_send![ctx, makeCurrentContext] };
            let swap_interval: c_int = 1;
            let _: () = unsafe {
                msg_send![
                    ctx,
                    setValues: &swap_interval as *const c_int,
                    forParameter: 222isize
                ]
            };
        }
        if let Some(player) = SHARED_PLAYER.get() {
            unsafe {
                let _ = player.ensure_opengl_render_context(gl_get_proc_address, std::ptr::null_mut());
            }
            let view_addr = this as usize;
            player.set_render_update_callback(Arc::new(move || unsafe {
                dispatch_async_f(
                    &_dispatch_main_q as *const c_void,
                    view_addr as *mut c_void,
                    trigger_view_redraw,
                );
            }));
        }
    }

    unsafe extern "C" fn draw_rect(this: *mut AnyObject, _sel: Sel, _dirty: CGRect) {
        let ctx: *mut AnyObject = unsafe { msg_send![this, openGLContext] };
        if ctx.is_null() {
            return;
        }
        let _: () = unsafe { msg_send![ctx, makeCurrentContext] };
        let bounds: CGRect = unsafe { msg_send![this, bounds] };
        let backing: CGRect = unsafe { msg_send![this, convertRectToBacking: bounds] };
        let w = backing.size.width.round().max(1.0) as i32;
        let h = backing.size.height.round().max(1.0) as i32;
        unsafe {
            glClearColor(0.0, 0.0, 0.0, 1.0);
            glClear(GL_COLOR_BUFFER_BIT);
        }
        if let Some(player) = SHARED_PLAYER.get() {
            player.render_gl_fbo(0, w, h, true);
        }
        let _: () = unsafe { msg_send![ctx, flushBuffer] };
    }

    unsafe extern "C" fn is_opaque(_this: *mut AnyObject, _sel: Sel) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn hit_test(_this: *mut AnyObject, _sel: Sel, _pt: CGPoint) -> *mut AnyObject {
        std::ptr::null_mut()
    }

    fn gl_view_class() -> &'static AnyClass {
        static VIEW_CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
        VIEW_CLASS.get_or_init(|| {
            if let Some(existing) = AnyClass::get(c"VesselMpvGlView") {
                return existing;
            }
            let super_cls = AnyClass::get(c"NSOpenGLView").expect("NSOpenGLView available");
            let mut builder =
                ClassBuilder::new(c"VesselMpvGlView", super_cls).expect("define VesselMpvGlView");
            unsafe {
                builder.add_method(
                    sel!(prepareOpenGL),
                    prepare_opengl as unsafe extern "C" fn(*mut AnyObject, Sel),
                );
                builder.add_method(
                    sel!(drawRect:),
                    draw_rect as unsafe extern "C" fn(*mut AnyObject, Sel, CGRect),
                );
                builder.add_method(
                    sel!(isOpaque),
                    is_opaque as unsafe extern "C" fn(*mut AnyObject, Sel) -> Bool,
                );
                builder.add_method(
                    sel!(hitTest:),
                    hit_test as unsafe extern "C" fn(*mut AnyObject, Sel, CGPoint) -> *mut AnyObject,
                );
            }
            builder.register()
        })
    }

    window
        .with_webview(move |webview| unsafe {
            let wk = webview.inner() as *mut AnyObject;
            if wk.is_null() {
                return;
            }
            let draws_bg_val: *mut AnyObject =
                msg_send![class!(NSNumber), numberWithBool: Bool::new(!visible)];
            let key: *mut AnyObject =
                msg_send![class!(NSString), stringWithUTF8String: c"drawsBackground".as_ptr()];
            let _: () = msg_send![wk, setValue: draws_bg_val, forKey: key];

            let mut view_ptr = MACOS_GL_VIEW.load(Ordering::Acquire) as *mut AnyObject;
            if view_ptr.is_null() && visible {
                let superview: *mut AnyObject = msg_send![wk, superview];
                if !superview.is_null() {
                    let bounds: CGRect = msg_send![superview, bounds];
                    let attrs: [u32; 5] = [
                        99,     // NSOpenGLPFAOpenGLProfile
                        0x3200, // NSOpenGLProfileVersion3_2Core
                        5,      // NSOpenGLPFADoubleBuffer
                        73,     // NSOpenGLPFAAccelerated
                        0,
                    ];
                    let pf_alloc: *mut AnyObject = msg_send![class!(NSOpenGLPixelFormat), alloc];
                    let mut pf: *mut AnyObject =
                        msg_send![pf_alloc, initWithAttributes: attrs.as_ptr()];
                    if pf.is_null() {
                        let fallback: [u32; 2] = [5, 0];
                        let pf_alloc2: *mut AnyObject =
                            msg_send![class!(NSOpenGLPixelFormat), alloc];
                        pf = msg_send![pf_alloc2, initWithAttributes: fallback.as_ptr()];
                    }
                    let view_alloc: *mut AnyObject = msg_send![gl_view_class(), alloc];
                    view_ptr = msg_send![view_alloc, initWithFrame: bounds, pixelFormat: pf];
                    if !pf.is_null() {
                        let _: () = msg_send![pf, release];
                    }
                    if !view_ptr.is_null() {
                        let _: () = msg_send![
                            view_ptr,
                            setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE
                        ];
                        let _: () = msg_send![view_ptr, setWantsBestResolutionOpenGLSurface: Bool::YES];
                        let _: () = msg_send![
                            superview,
                            addSubview: view_ptr,
                            positioned: NS_WINDOW_BELOW,
                            relativeTo: wk
                        ];
                        MACOS_GL_VIEW.store(view_ptr as usize, Ordering::Release);
                    }
                }
            }

            if !view_ptr.is_null() {
                let _: () = msg_send![view_ptr, setHidden: Bool::new(!visible)];
                if visible {
                    let _: () = msg_send![view_ptr, setNeedsDisplay: Bool::YES];
                }
            }

            let ns_win = webview.ns_window() as *mut AnyObject;
            if !ns_win.is_null() {
                let color: *mut AnyObject = if visible {
                    msg_send![class!(NSColor), blackColor]
                } else {
                    msg_send![class!(NSColor), clearColor]
                };
                let _: () = msg_send![ns_win, setBackgroundColor: color];
            }
        })
        .map_err(|err| VesselError::Player(format!("update macOS embedded surface: {err}")))
}

#[cfg(target_os = "linux")]
fn linux_set_surface_visible(
    window: &WebviewWindow,
    player: &Arc<MpvEmbeddedPlayer>,
    visible: bool,
) -> VesselResult<()> {
    use std::ffi::{c_char, c_void};

    use gtk::gdk;
    use gtk::glib;
    use gtk::prelude::*;
    use webkit2gtk::WebViewExt;

    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    unsafe extern "C" fn linux_get_proc_address(
        _ctx: *mut c_void,
        name: *const c_char,
    ) -> *mut c_void {
        if name.is_null() {
            return std::ptr::null_mut();
        }
        let rtld_default: *mut c_void = std::ptr::null_mut();
        let egl_sym = unsafe { dlsym(rtld_default, c"eglGetProcAddress".as_ptr()) };
        if !egl_sym.is_null() {
            let egl_get_proc: unsafe extern "C" fn(*const c_char) -> *mut c_void =
                unsafe { std::mem::transmute(egl_sym) };
            let ptr = unsafe { egl_get_proc(name) };
            if !ptr.is_null() {
                return ptr;
            }
        }
        let glx_sym = unsafe { dlsym(rtld_default, c"glXGetProcAddressARB".as_ptr()) };
        if !glx_sym.is_null() {
            let glx_get_proc: unsafe extern "C" fn(*const c_char) -> *mut c_void =
                unsafe { std::mem::transmute(glx_sym) };
            let ptr = unsafe { glx_get_proc(name) };
            if !ptr.is_null() {
                return ptr;
            }
        }
        unsafe { dlsym(rtld_default, name) }
    }

    let player_ref = Arc::clone(player);
    window
        .with_webview(move |webview| {
            let wv = webview.inner();
            let bg = if visible {
                gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)
            } else {
                gdk::RGBA::new(0.118, 0.118, 0.180, 1.0)
            };
            wv.set_background_color(&bg);

            let Some(parent) = wv.parent() else {
                return;
            };
            if let Ok(overlay) = parent.clone().downcast::<gtk::Overlay>() {
                for child in overlay.children() {
                    if let Ok(gl_area) = child.downcast::<gtk::GLArea>() {
                        if visible {
                            gl_area.show();
                            gl_area.queue_render();
                        } else {
                            gl_area.hide();
                        }
                        return;
                    }
                }
            }

            if !visible {
                return;
            }

            let Ok(container) = parent.downcast::<gtk::Container>() else {
                return;
            };
            let gl_area = gtk::GLArea::new();
            gl_area.set_hexpand(true);
            gl_area.set_vexpand(true);
            gl_area.set_auto_render(false);
            gl_area.set_can_focus(false);

            let p_realize = Arc::clone(&player_ref);
            let gl_weak = glib::SendWeakRef::from(gl_area.downgrade());
            gl_area.connect_realize(move |area| {
                area.make_current();
                if area.error().is_some() {
                    return;
                }
                unsafe {
                    let _ = p_realize.ensure_opengl_render_context(
                        linux_get_proc_address,
                        std::ptr::null_mut(),
                    );
                }
                let weak_cb = gl_weak.clone();
                p_realize.set_render_update_callback(Arc::new(move || {
                    let weak = weak_cb.clone();
                    glib::idle_add_once(move || {
                        if let Some(area) = weak.upgrade() {
                            area.queue_render();
                        }
                    });
                }));
            });

            let p_render = Arc::clone(&player_ref);
            gl_area.connect_render(move |area, _ctx| {
                let scale = area.scale_factor().max(1);
                let w = (area.allocated_width() * scale).max(1);
                let h = (area.allocated_height() * scale).max(1);
                p_render.render_gl_fbo(0, w, h, true);
                glib::Propagation::Stop
            });

            let overlay = gtk::Overlay::new();
            container.remove(&wv);
            overlay.add(&gl_area);
            overlay.add_overlay(&wv);
            overlay.set_overlay_pass_through(&gl_area, true);
            container.add(&overlay);
            overlay.show_all();
        })
        .map_err(|err| VesselError::Player(format!("update Linux embedded surface: {err}")))
}

#[cfg(target_os = "windows")]
fn windows_set_surface_visible(
    window: &WebviewWindow,
    player: &Arc<MpvEmbeddedPlayer>,
    visible: bool,
) -> VesselResult<()> {
    use windows::Win32::Foundation::{COLORREF, HWND, RECT};
    use windows::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, GetClientRect, HWND_BOTTOM, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SetWindowPos,
        ShowWindow, WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
    };

    let player_ref = Arc::clone(player);
    let parent_hwnd_raw = window
        .hwnd()
        .map_err(|err| VesselError::Player(format!("resolve Windows HWND: {err}")))?
        .0 as isize;
    window
        .with_webview(move |_webview| {
            let parent_hwnd = HWND(parent_hwnd_raw as *mut std::ffi::c_void);
            let mut rect = RECT::default();
            unsafe {
                let _ = GetClientRect(parent_hwnd, &mut rect);
            }
            if visible {
                let width = (rect.right - rect.left).max(1);
                let height = (rect.bottom - rect.top).max(1);
                let static_cls: Vec<u16> = "STATIC\0".encode_utf16().collect();
                let child = unsafe {
                    CreateWindowExW(
                        Default::default(),
                        windows::core::PCWSTR(static_cls.as_ptr()),
                        windows::core::PCWSTR::null(),
                        WS_CHILD | WS_CLIPSIBLINGS | WS_VISIBLE,
                        0,
                        0,
                        width,
                        height,
                        Some(parent_hwnd),
                        None,
                        None,
                        None,
                    )
                };
                if let Ok(hwnd) = child {
                    unsafe {
                        let _ = CreateSolidBrush(COLORREF(0));
                        let _ = SetWindowPos(hwnd, Some(HWND_BOTTOM), 0, 0, width, height, SWP_NOACTIVATE);
                        let _ = ShowWindow(hwnd, SW_SHOW);
                    }
                    player_ref.set_wid_target(Some(hwnd.0 as isize as i64));
                }
            } else {
                player_ref.set_wid_target(None);
                let _ = (HWND(std::ptr::null_mut()), SW_HIDE);
            }
        })
        .map_err(|err| VesselError::Player(format!("update Windows embedded surface: {err}")))
}
