//! Native's window services on Linux and the BSDs, through the protocols of Wayland and X11
//! compositors. Requests to Wayland compositors go through Qt's connection, as they refer to its
//! windows' surfaces, with the libwayland-client that Qt has loaded rather than one that the app
//! links to. Requests to X11 servers go through a connection of the app's own.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("manderrow/cpp/unix.h");
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        type QGuiApplication;
        type QWindow = crate::qt::native::qobject::QWindow;
        type WId = super::WId;

        type wl_display;
        type wl_proxy;
        type wl_seat;
        #[namespace = "QNativeInterface"]
        type QWaylandApplication;

        fn display(self: &QWaylandApplication) -> *mut wl_display;
        #[rust_name = "last_input_seat"]
        fn lastInputSeat(self: &QWaylandApplication) -> *mut wl_seat;
        #[rust_name = "last_input_serial"]
        fn lastInputSerial(self: &QWaylandApplication) -> u32;

        #[Self = "QGuiApplication"]
        #[rust_name = "platform_name"]
        fn platformName() -> QString;

        #[rust_name = "request_update"]
        fn requestUpdate(self: Pin<&mut QWindow>);
        #[rust_name = "win_id"]
        fn winId(self: &QWindow) -> WId;
    }

    #[namespace = "manderrow"]
    unsafe extern "C++" {
        #[rust_name = "wayland_application"]
        fn waylandApplication() -> *mut QWaylandApplication;
        #[rust_name = "wayland_window_resource"]
        unsafe fn waylandWindowResource(
            window: Pin<&mut QWindow>,
            name: *const c_char,
        ) -> *mut wl_proxy;
    }
}

use core::cell::RefCell;
use core::ffi::CStr;
use core::mem::ManuallyDrop;
use core::pin::Pin;
use std::collections::HashMap;

use cxx_qt_lib::QString;
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::protocol::{wl_compositor, wl_region, wl_registry, wl_seat, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1 as ext_manager,
    ext_background_effect_surface_v1 as ext_surface,
};
use wayland_protocols::xdg::shell::client::xdg_toplevel;
use wayland_protocols_plasma::blur::client::{org_kde_kwin_blur, org_kde_kwin_blur_manager};
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use ffi::QWindow;

/// A window's platform handle, which is its window on X11.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct WId(usize);

// SAFETY: WId is quintptr, an unsigned integer the size of a pointer
unsafe impl cxx::ExternType for WId {
    type Id = cxx::type_id!("WId");
    type Kind = cxx::kind::Trivial;
}

/// Wraps one of Qt's Wayland objects, which Qt owns and dispatches the events of.
fn foreign<P: Proxy>(connection: &Connection, proxy: *mut ffi::wl_proxy) -> Option<P> {
    if proxy.is_null() {
        return None;
    }
    // SAFETY: the proxy is a live object of P's interface, which Qt keeps for at least as long as
    // the requests sent here
    let id = unsafe { ObjectId::from_ptr(P::interface(), proxy.cast()) }.ok()?;
    P::from_id(connection, id).ok()
}

fn window_resource<P: Proxy>(
    connection: &Connection,
    window: Pin<&mut QWindow>,
    name: &CStr,
) -> Option<P> {
    // SAFETY: the name is a C string
    foreign(connection, unsafe {
        ffi::wayland_window_resource(window, name.as_ptr())
    })
}

/// The Wayland globals that the app binds, from its own event queue, so that its events are never
/// dispatched by Qt's.
#[derive(Default)]
struct Globals {
    compositor: Option<(u32, wl_compositor::WlCompositor)>,
    /// The standard protocol for blur, as of wayland-protocols 1.45
    background_effect: Option<(u32, ext_manager::ExtBackgroundEffectManagerV1)>,
    background_effect_blurs: bool,
    /// KWin's protocol for blur, which it only offers while its blur effect is enabled
    kde_blur: Option<(u32, org_kde_kwin_blur_manager::OrgKdeKwinBlurManager)>,
}

/// A window's blur, which lasts for as long as the object does.
enum Blur {
    Background(ext_surface::ExtBackgroundEffectSurfaceV1),
    Kde(org_kde_kwin_blur::OrgKdeKwinBlur),
}

impl Blur {
    fn destroy(self) {
        match self {
            Self::Background(effect) => effect.destroy(),
            Self::Kde(blur) => blur.release(),
        }
    }
}

struct Wayland {
    connection: Connection,
    queue: EventQueue<Globals>,
    globals: Globals,
    /// The blur of each window that has one, by its address
    blurs: HashMap<usize, Blur>,
}

impl Wayland {
    fn new(display: *mut ffi::wl_display) -> Option<Self> {
        if display.is_null() {
            return None;
        }
        // SAFETY: the display is Qt's, which lives as long as the app
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let mut queue = connection.new_event_queue();
        connection.display().get_registry(&queue.handle(), ());
        let mut globals = Globals::default();
        // one round trip for the globals, and one for the events sent as they are bound
        for _ in 0..2 {
            if let Err(e) = queue.roundtrip(&mut globals) {
                slog_scope::warn!("Failed to list the Wayland globals: {e}");
                return None;
            }
        }
        Some(Self {
            connection,
            queue,
            globals,
            blurs: HashMap::new(),
        })
    }

    fn set_blur_behind(&mut self, mut window: Pin<&mut QWindow>, enabled: bool) -> bool {
        // globals that have come or gone since
        if let Err(e) = self.queue.dispatch_pending(&mut self.globals) {
            slog_scope::warn!("Failed to dispatch Wayland events: {e}");
        }
        let key = std::ptr::from_ref(&*window) as usize;
        // The window may have a new surface since, as Qt replaces it whenever the window is
        // hidden, so the blur is made anew each time, and a surface can only have one at a time.
        let had_blur = self.blurs.remove(&key).map(Blur::destroy).is_some();
        let Some(surface) =
            window_resource::<wl_surface::WlSurface>(&self.connection, window.as_mut(), c"surface")
        else {
            return false;
        };
        let qh = self.queue.handle();
        let globals = &self.globals;
        let blur = if !enabled {
            if let Some((_, manager)) = &globals.kde_blur {
                manager.unset(&surface);
            }
            None
        } else if let (Some((_, manager)), Some((_, compositor)), true) = (
            &globals.background_effect,
            &globals.compositor,
            globals.background_effect_blurs,
        ) {
            // the region is clipped to the surface, so this covers all of it whatever its size
            let region = compositor.create_region(&qh, ());
            region.add(0, 0, i32::MAX, i32::MAX);
            let effect = manager.get_background_effect(&surface, &qh, ());
            effect.set_blur_region(Some(&region));
            region.destroy();
            Some(Blur::Background(effect))
        } else if let Some((_, manager)) = &globals.kde_blur {
            let blur = manager.create(&surface, &qh, ());
            // no region is the whole surface
            blur.set_region(None);
            blur.commit();
            Some(Blur::Kde(blur))
        } else {
            return false;
        };
        let applied = blur.is_some() == enabled;
        if let Some(blur) = blur {
            self.blurs.insert(key, blur);
        }
        if let Err(e) = self.connection.flush() {
            slog_scope::warn!("Failed to flush the Wayland connection: {e}");
        }
        // the blur applies once the surface is next committed, which Qt does with each frame
        if enabled || had_blur {
            window.request_update();
        }
        applied
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Globals {
    fn event(
        globals: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == wl_compositor::WlCompositor::interface().name {
                    globals.compositor = Some((name, registry.bind(name, version.min(4), qh, ())));
                } else if interface == ext_manager::ExtBackgroundEffectManagerV1::interface().name {
                    globals.background_effect = Some((name, registry.bind(name, 1, qh, ())));
                } else if interface
                    == org_kde_kwin_blur_manager::OrgKdeKwinBlurManager::interface().name
                {
                    globals.kde_blur = Some((name, registry.bind(name, 1, qh, ())));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if globals.compositor.as_ref().is_some_and(|(n, _)| *n == name) {
                    globals.compositor = None;
                }
                if let Some((_, manager)) = globals.background_effect.take_if(|(n, _)| *n == name) {
                    manager.destroy();
                    globals.background_effect_blurs = false;
                }
                // which has no destructor
                globals.kde_blur.take_if(|(n, _)| *n == name);
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_manager::ExtBackgroundEffectManagerV1, ()> for Globals {
    fn event(
        globals: &mut Self,
        _: &ext_manager::ExtBackgroundEffectManagerV1,
        event: ext_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_manager::Event::Capabilities { flags } = event {
            globals.background_effect_blurs = flags
                .into_result()
                .is_ok_and(|flags| flags.contains(ext_manager::Capability::Blur));
        }
    }
}

/// The objects without events
macro_rules! ignore_events {
    ($($proxy:ty),*) => {$(
        impl Dispatch<$proxy, ()> for Globals {
            fn event(
                _: &mut Self,
                _: &$proxy,
                _: <$proxy as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

ignore_events!(
    wl_compositor::WlCompositor,
    wl_region::WlRegion,
    ext_surface::ExtBackgroundEffectSurfaceV1,
    org_kde_kwin_blur_manager::OrgKdeKwinBlurManager,
    org_kde_kwin_blur::OrgKdeKwinBlur
);

thread_local! {
    /// The app's own view of Qt's Wayland connection, once a window has needed it. It is never
    /// dropped, as its objects would be destroyed after Qt has closed the connection.
    static WAYLAND: ManuallyDrop<RefCell<Option<Option<Wayland>>>> =
        const { ManuallyDrop::new(RefCell::new(None)) };
}

/// Runs `f` with the app's view of Qt's Wayland connection, if Qt is running on Wayland.
fn with_wayland<T>(f: impl FnOnce(&mut Wayland) -> T) -> Option<T> {
    WAYLAND.with(|wayland| {
        wayland
            .borrow_mut()
            .get_or_insert_with(|| {
                // SAFETY: the app's Wayland connection lives as long as the app
                let app = unsafe { ffi::wayland_application().as_ref() }?;
                Wayland::new(app.display())
            })
            .as_mut()
            .map(f)
    })
}

struct X11 {
    connection: RustConnection,
    /// What KWin blurs behind a window by: a list of rectangles, which is the whole window when
    /// empty
    blur_atom: u32,
}

impl X11 {
    fn new() -> Option<Self> {
        if ffi::QGuiApplication::platform_name() != QString::from("xcb") {
            return None;
        }
        // to the display that Qt connected to, as it does by DISPLAY
        let (connection, _) = x11rb::connect(None)
            .inspect_err(|e| slog_scope::warn!("Failed to connect to the X server: {e}"))
            .ok()?;
        let blur_atom = connection
            .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")
            .ok()?
            .reply()
            .ok()?
            .atom;
        Some(Self {
            connection,
            blur_atom,
        })
    }

    fn set_blur_behind(&self, window: u32, enabled: bool) -> bool {
        let sent = if enabled {
            self.connection
                .change_property32(
                    PropMode::REPLACE,
                    window,
                    self.blur_atom,
                    AtomEnum::CARDINAL,
                    &[],
                )
                .map(drop)
        } else {
            self.connection
                .delete_property(window, self.blur_atom)
                .map(drop)
        };
        sent.is_ok() && self.connection.flush().is_ok()
    }
}

thread_local! {
    static X11: Option<X11> = X11::new();
}

pub fn set_blur_behind(mut window: Pin<&mut QWindow>, enabled: bool) -> bool {
    if let Some(applied) = with_wayland(|wayland| wayland.set_blur_behind(window.as_mut(), enabled))
    {
        return applied;
    }
    X11.with(|x11| {
        let window = u32::try_from(window.win_id().0);
        x11.as_ref()
            .zip(window.ok())
            .is_some_and(|(x11, window)| x11.set_blur_behind(window, enabled))
    })
}

pub fn set_dark_frame(_window: Pin<&mut QWindow>, _dark: bool) -> bool {
    false
}

pub fn hide_title_bar(_window: Pin<&mut QWindow>) -> bool {
    false
}

pub fn show_window_menu(window: Pin<&mut QWindow>, x: f64, y: f64) -> bool {
    // SAFETY: the app's Wayland connection lives as long as the app
    let Some(app) = (unsafe { ffi::wayland_application().as_ref() }) else {
        return false;
    };
    with_wayland(|wayland| {
        let connection = &wayland.connection;
        let Some(toplevel) =
            window_resource::<xdg_toplevel::XdgToplevel>(connection, window, c"xdg_toplevel")
        else {
            return false;
        };
        let Some(seat) = foreign::<wl_seat::WlSeat>(connection, app.last_input_seat().cast())
        else {
            return false;
        };
        toplevel.show_window_menu(&seat, app.last_input_serial(), x as i32, y as i32);
        connection.flush().is_ok()
    })
    .unwrap_or(false)
}
