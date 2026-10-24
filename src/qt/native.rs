//! The `Native` QML singleton, which provides the platform services that QML can't provide on its
//! own.

#[cxx_qt::bridge]
pub mod qobject {
    #[namespace = "Qt"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/qt.h");
        type KeyboardModifiers = cxx_qt_lib::KeyboardModifiers;
        type MouseButton = cxx_qt_lib::MouseButton;
        type MouseButtons = cxx_qt_lib::MouseButtons;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qpointf.h");
        type QPointF = cxx_qt_lib::QPointF;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qtypes.h");
        type qsizetype = cxx_qt_lib::qsizetype;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;

        include!(<QtCore/QCoreApplication>);
        type QCoreApplication;
        include!(<QtGui/QDesktopServices>);
        type QDesktopServices;
        include!(<QtGui/QGuiApplication>);
        type QGuiApplication;
        include!(<QtGui/QKeyEvent>);
        type QKeyEvent;
        include!(<QtGui/QMouseEvent>);
        type QMouseEvent;

        include!("manderrow/src/cpp/qt.h");
        type QClipboard;
        type QEvent;
        type QJSValue;
        type QObjectList;

        // QImage and QObject, as types that the bindings here can add methods to, which they
        // can't to cxx-qt's
        #[cxx_name = "QImage"]
        type Image;
        #[cxx_name = "QObject"]
        type Object;
    }

    unsafe extern "C++Qt" {
        include!(<QtGui/QWindow>);
        #[qobject]
        #[base = QObject]
        type QWindow;

        include!(<QtQuick/QQuickWindow>);
        #[qobject]
        #[base = QWindow]
        type QQuickWindow;

        include!(<QtQuick/QQuickItem>);
        #[qobject]
        #[base = QObject]
        type QQuickItem;
    }

    /// The types of events that Native handles.
    #[namespace = "manderrow"]
    #[repr(u32)]
    enum QEventType {
        MouseButtonPress = 2,
        MouseButtonRelease = 3,
        MouseMove = 5,
        KeyPress = 6,
        KeyRelease = 7,
        WindowStateChange = 105,
        Expose = 206,
        ApplicationStateChange = 214,
    }

    #[namespace = "manderrow"]
    #[repr(u32)]
    enum QClipboardMode {
        Clipboard = 0,
    }

    #[namespace = "manderrow"]
    #[repr(u32)]
    enum QQuickItemFlag {
        ItemHasContents = 0x08,
    }

    #[namespace = "manderrow"]
    unsafe extern "C++" {
        type QEventType;
        type QClipboardMode;
        type QQuickItemFlag;
        type QQuickItemFlags = super::QQuickItemFlags;
        type QQuickItemList;

        #[rust_name = "child_items"]
        fn childItems(item: &QQuickItem) -> UniquePtr<QQuickItemList>;

        #[rust_name = "call_with_bool"]
        fn callWithBool(function: &QJSValue, argument: bool) -> bool;

        #[rust_name = "hover_link"]
        fn hoverLink(
            edit: &QQuickItem,
            position: i32,
            href: &QString,
            link_color: &QColor,
            hover_color: &QColor,
        );
    }

    unsafe extern "C++" {
        #[Self = "QCoreApplication"]
        #[rust_name = "core_application"]
        fn instance() -> *mut QCoreApplication;

        #[rust_name = "install_event_filter"]
        unsafe fn installEventFilter(self: Pin<&mut QCoreApplication>, filter: *mut QObject);

        #[Self = "QCoreApplication"]
        #[rust_name = "send_event"]
        unsafe fn sendEvent(receiver: *mut QObject, event: *mut QEvent) -> bool;

        #[Self = "QGuiApplication"]
        fn clipboard() -> *mut QClipboard;

        #[Self = "QGuiApplication"]
        #[rust_name = "keyboard_modifiers"]
        fn keyboardModifiers() -> KeyboardModifiers;

        #[Self = "QGuiApplication"]
        #[rust_name = "query_keyboard_modifiers"]
        fn queryKeyboardModifiers() -> KeyboardModifiers;

        #[rust_name = "set_text"]
        fn setText(self: Pin<&mut QClipboard>, text: &QString, mode: QClipboardMode);

        #[Self = "QDesktopServices"]
        #[rust_name = "open_url"]
        fn openUrl(url: &QUrl) -> bool;

        unsafe fn save(
            self: &Image,
            file_name: &QString,
            format: *const c_char,
            quality: i32,
        ) -> bool;

        unsafe fn inherits(self: &Object, class_name: *const c_char) -> bool;
        fn size(self: &QObjectList) -> qsizetype;
        fn value(self: &QObjectList, i: qsizetype) -> *mut Object;
        fn size(self: &QQuickItemList) -> qsizetype;
        fn value(self: &QQuickItemList, i: qsizetype) -> *mut QQuickItem;

        #[cxx_name = "type"]
        fn event_type(self: &QEvent) -> QEventType;
        fn key(self: &QKeyEvent) -> i32;
        fn modifiers(self: &QKeyEvent) -> KeyboardModifiers;
        #[rust_name = "scene_position"]
        fn scenePosition(self: &QMouseEvent) -> QPointF;

        #[rust_name = "map_to_global"]
        fn mapToGlobal(self: &QWindow, pos: &QPointF) -> QPointF;

        #[rust_name = "grab_window"]
        fn grabWindow(self: Pin<&mut QQuickWindow>) -> QImage;
        #[rust_name = "content_item"]
        fn contentItem(self: &QQuickWindow) -> *mut QQuickItem;

        #[rust_name = "is_visible"]
        fn isVisible(self: &QQuickItem) -> bool;
        unsafe fn inherits(self: &QQuickItem, class_name: *const c_char) -> bool;
        fn children(self: &QQuickItem) -> &QObjectList;
        fn contains(self: &QQuickItem, point: &QPointF) -> bool;
        #[rust_name = "map_from_scene"]
        fn mapFromScene(self: &QQuickItem, point: &QPointF) -> QPointF;
        fn clip(self: &QQuickItem) -> bool;
        fn z(self: &QQuickItem) -> f64;
        fn flags(self: &QQuickItem) -> QQuickItemFlags;
        #[rust_name = "accepted_mouse_buttons"]
        fn acceptedMouseButtons(self: &QQuickItem) -> MouseButtons;
        #[rust_name = "accept_hover_events"]
        fn acceptHoverEvents(self: &QQuickItem) -> bool;
        #[rust_name = "parent_item"]
        fn parentItem(self: &QQuickItem) -> *mut QQuickItem;

        #[rust_name = "is_callable"]
        fn isCallable(self: &QJSValue) -> bool;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "qjsvalue_clone"]
        fn make_unique(value: &QJSValue) -> UniquePtr<QJSValue>;

        #[rust_name = "qmouseevent_new"]
        fn make_unique(
            event_type: QEventType,
            local_pos: &QPointF,
            scene_pos: &QPointF,
            global_pos: &QPointF,
            button: MouseButton,
            buttons: MouseButtons,
            modifiers: KeyboardModifiers,
        ) -> UniquePtr<QMouseEvent>;

        #[rust_name = "qkeyevent_new"]
        fn make_unique(
            event_type: QEventType,
            key: i32,
            modifiers: KeyboardModifiers,
            text: &QString,
        ) -> UniquePtr<QKeyEvent>;
    }

    #[namespace = "rust::cxxqt1"]
    unsafe extern "C++" {
        include!("cxx-qt/casting.h");

        #[rust_name = "upcast_key_event"]
        unsafe fn upcastPtr(event: *const QKeyEvent) -> *const QEvent;
        #[rust_name = "downcast_key_event"]
        unsafe fn downcastPtr(event: *const QEvent) -> *const QKeyEvent;
        #[rust_name = "upcast_mouse_event"]
        unsafe fn upcastPtr(event: *const QMouseEvent) -> *const QEvent;
        #[rust_name = "downcast_mouse_event"]
        unsafe fn downcastPtr(event: *const QEvent) -> *const QMouseEvent;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Whether the Shift key is currently held down, anywhere in the application.
        #[qproperty(bool, shift, READ = shift, NOTIFY = modifiers_changed)]
        /// Whether the Control key is currently held down, anywhere in the application.
        #[qproperty(bool, ctrl, READ = ctrl, NOTIFY = modifiers_changed)]
        type Native = super::NativeRust;

        fn shift(self: &Native) -> bool;
        fn ctrl(self: &Native) -> bool;

        #[qinvokable]
        fn copy_text(self: &Native, text: &QString);

        /// Opens a folder in the platform's file manager. Returns whether it could.
        #[qinvokable]
        fn open_folder(self: &Native, path: &QString) -> bool;

        /// Saves a screenshot of a window, including its popups. Returns whether it succeeded.
        #[qinvokable]
        unsafe fn grab_window(self: &Native, window: *mut QQuickWindow, path: &QString) -> bool;

        /// Clicks a window at a point in its coordinates, as the mouse would. For development
        /// scripts.
        #[qinvokable]
        unsafe fn click(self: &Native, window: *mut QQuickWindow, x: f64, y: f64);

        /// Drags the mouse across a window, between points in its coordinates. For development
        /// scripts.
        #[qinvokable]
        unsafe fn drag(
            self: &Native,
            window: *mut QQuickWindow,
            from_x: f64,
            from_y: f64,
            to_x: f64,
            to_y: f64,
        );

        /// Presses and releases a key in a window, as the keyboard would, with `modifiers` (a
        /// `Qt.KeyboardModifiers` value) held, typing `text`. For development scripts.
        #[qinvokable]
        unsafe fn key(
            self: &Native,
            window: *mut QQuickWindow,
            key: i32,
            modifiers: i32,
            text: &QString,
        );

        /// The topmost item at a point of a window, in scene coordinates, that is drawn or takes
        /// input, as a browser finds the element under the pointer. Items that only arrange or
        /// hold others, like layouts and Loaders, don't count.
        #[qinvokable]
        unsafe fn item_at(
            self: &Native,
            window: *mut QQuickWindow,
            x: f64,
            y: f64,
        ) -> *mut QQuickItem;

        /// Blurs what is behind a translucent window, where the platform can: with Wayland
        /// compositors that support ext-background-effect or KWin's blur, with KWin on X11, on
        /// Windows 10 and 11, with an acrylic blur, and on macOS, once the window is shown.
        /// Returns whether the platform can.
        #[qinvokable]
        unsafe fn set_blur_behind(self: &Native, window: *mut QQuickWindow, enabled: bool) -> bool;

        /// Tints the platform's parts of a window, like its border on Windows, for a
        /// dark or light theme. Returns whether the platform can: on Windows and macOS.
        #[qinvokable]
        unsafe fn set_dark_frame(self: &Native, window: *mut QQuickWindow, dark: bool) -> bool;

        /// Hides the title bar of a window that has a native frame, keeping the rest of the
        /// frame, so that the app can draw its own title bar. Call it before the window is
        /// shown. Returns whether the platform can: on Windows, where a window keeps its resize
        /// borders, shadow, snapping, and animations this way, which frameless windows lack, and
        /// on macOS, where the window keeps its buttons, which `place_window_buttons` places, and
        /// must have the `ExpandedClientAreaHint` and `NoTitleBarBackgroundHint` flags.
        #[qinvokable]
        unsafe fn hide_title_bar(self: &Native, window: *mut QQuickWindow) -> bool;

        /// Shows the window manager's menu for a window, as right clicking a native title bar
        /// does, at a point in the window's coordinates. Call it when the right button is
        /// pressed. Returns whether the platform can: on Wayland, when Qt's private platform
        /// headers were available to the build, and on Windows.
        #[qinvokable]
        unsafe fn show_window_menu(
            self: &Native,
            window: *mut QQuickWindow,
            x: f64,
            y: f64,
        ) -> bool;

        /// The edges of a window that the window manager has tiled, as bits: 1 for the left, 2
        /// for the right, 4 for the top, and 8 for the bottom. Tiling window managers tile every
        /// edge of the windows they tile, and others may tile some, as for a window snapped to
        /// half of the screen, or all, as for a maximized one. It changes as `window_configured`
        /// is emitted. The platform tells it on Wayland, with compositors that support xdg-shell
        /// 2, when Qt's private platform headers were available to the build. Elsewhere it is 0.
        #[qinvokable]
        unsafe fn tiled_edges(self: &Native, window: *mut QQuickWindow) -> i32;

        /// Places the buttons of a window's native title bar in the app's title bar, which is
        /// `bar_height` tall at the top of the window, centered in its height, and keeps them
        /// there. Returns how much of the bar they take from its left edge, with space after
        /// them, or 0 if the window has no such buttons there: on macOS, in full screen, and on
        /// other platforms.
        #[qinvokable]
        unsafe fn place_window_buttons(
            self: &Native,
            window: *mut QQuickWindow,
            bar_height: f64,
        ) -> f64;

        /// Shows the link with `href` at a position in a rich text TextEdit as hovered, as a
        /// browser styles `a:hover`: underlined, and in `hover_color` where it is in
        /// `link_color`. The link shown as hovered before in it goes back to how it was. An empty
        /// `href` shows none as hovered.
        #[qinvokable]
        unsafe fn hover_link(
            self: &Native,
            edit: *mut QQuickItem,
            position: i32,
            href: &QString,
            link_color: &QColor,
            hover_color: &QColor,
        );

        /// Takes over moving the focus with Tab and Shift+Tab in every window: `handler(forward)`
        /// is called before the key reaches any item, and the key goes no further if it returns
        /// true.
        #[qinvokable]
        fn set_tab_handler(self: Pin<&mut Native>, handler: &QJSValue);

        #[cxx_override]
        unsafe fn event_filter(
            self: Pin<&mut Native>,
            watched: *mut QObject,
            event: *mut QEvent,
        ) -> bool;

        #[qsignal]
        fn modifiers_changed(self: Pin<&mut Native>);

        /// A mouse button was pressed in a window, at a point in its coordinates. This is emitted
        /// before the press is delivered to the window's items.
        #[qsignal]
        fn pointer_pressed(self: Pin<&mut Native>, position: QPointF);

        /// A key was pressed in a window, before it is delivered to the focused item.
        #[qsignal]
        fn key_pressed(self: Pin<&mut Native>, key: i32);

        /// The window manager may have changed the state of a window, such as by tiling it: a
        /// window was exposed or changed its state, as Qt does whenever the window manager
        /// configures one, such as a Wayland compositor with xdg_toplevel's configure event.
        #[qsignal]
        fn window_configured(self: Pin<&mut Native>);
    }

    impl cxx_qt::Initialize for Native {}
}

use core::pin::Pin;

use cxx::UniquePtr;
use cxx_qt::casting::{Downcast, Upcast};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    KeyboardModifier, KeyboardModifiers, MouseButton, MouseButtons, QColor, QFlag, QFlags,
    QPointF, QString, QUrl,
};

use qobject::{
    Image, QEvent, QEventType, QJSValue, QKeyEvent, QMouseEvent, QObject, QQuickItem,
    QQuickItemFlag, QQuickWindow, QWindow,
};

pub type QQuickItemFlags = QFlags<QQuickItemFlag>;

// SAFETY: QQuickItem::Flags is the QFlags of QQuickItem::Flag, which is 32 bits and unsigned
unsafe impl QFlag for QQuickItemFlag {
    type TypeId = cxx::type_id!("manderrow::QQuickItemFlags");
    type Repr = u32;

    fn to_repr(self) -> u32 {
        self.repr
    }
}

// SAFETY: the casts are C++'s, between QEvent and the event types that derive from it
unsafe impl Upcast<QEvent> for QKeyEvent {
    unsafe fn upcast_ptr(this: *const Self) -> *const QEvent {
        unsafe { qobject::upcast_key_event(this) }
    }

    unsafe fn from_base_ptr(base: *const QEvent) -> *const Self {
        unsafe { qobject::downcast_key_event(base) }
    }
}

// SAFETY: as above
unsafe impl Upcast<QEvent> for QMouseEvent {
    unsafe fn upcast_ptr(this: *const Self) -> *const QEvent {
        unsafe { qobject::upcast_mouse_event(this) }
    }

    unsafe fn from_base_ptr(base: *const QEvent) -> *const Self {
        unsafe { qobject::downcast_mouse_event(base) }
    }
}

cxx_qt::impl_transitive_cast!(QQuickWindow, QWindow, QObject);

const KEY_TAB: i32 = 0x01000001;
const KEY_BACKTAB: i32 = 0x01000002;

pub struct NativeRust {
    modifiers: KeyboardModifiers,
    tab_handler: UniquePtr<QJSValue>,
}

impl Default for NativeRust {
    fn default() -> Self {
        Self {
            modifiers: KeyboardModifiers::default(),
            tab_handler: UniquePtr::null(),
        }
    }
}

impl cxx_qt::Initialize for qobject::Native {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().modifiers = qobject::QGuiApplication::query_keyboard_modifiers();
        // SAFETY: the application outlives every QML singleton, and an event filter is removed
        // when it is destroyed
        unsafe {
            let filter: *mut QObject = Upcast::<QObject>::upcast_pin(self).get_unchecked_mut();
            if let Some(app) = qobject::QCoreApplication::core_application().as_mut() {
                Pin::new_unchecked(app).install_event_filter(filter);
            }
        }
    }
}

impl qobject::Native {
    fn shift(&self) -> bool {
        self.modifiers.test_flag(KeyboardModifier::ShiftModifier)
    }

    fn ctrl(&self) -> bool {
        self.modifiers.test_flag(KeyboardModifier::ControlModifier)
    }

    fn copy_text(&self, text: &QString) {
        // SAFETY: the application's clipboard lives as long as the application
        if let Some(clipboard) = unsafe { qobject::QGuiApplication::clipboard().as_mut() } {
            // SAFETY: the clipboard is a QObject, which doesn't move
            let clipboard = unsafe { Pin::new_unchecked(clipboard) };
            clipboard.set_text(text, qobject::QClipboardMode::Clipboard);
        }
    }

    fn open_folder(&self, path: &QString) -> bool {
        qobject::QDesktopServices::open_url(&QUrl::from_local_file(path))
    }

    unsafe fn grab_window(&self, window: *mut QQuickWindow, path: &QString) -> bool {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return false;
        };
        let image = window.grab_window();
        // SAFETY: cxx-qt's QImage is the C++ QImage, and a null format picks it from the file's
        // extension
        unsafe {
            let image = &*std::ptr::from_ref(&image).cast::<Image>();
            image.save(path, std::ptr::null(), -1)
        }
    }

    unsafe fn click(&self, window: *mut QQuickWindow, x: f64, y: f64) {
        let Some(mut window) = (unsafe { pin_window(window) }) else {
            return;
        };
        let point = QPointF::new(x, y);
        send_mouse(
            window.as_mut(),
            QEventType::MouseMove,
            &point,
            MouseButton::NoButton,
        );
        send_mouse(
            window.as_mut(),
            QEventType::MouseButtonPress,
            &point,
            MouseButton::LeftButton,
        );
        send_mouse(
            window,
            QEventType::MouseButtonRelease,
            &point,
            MouseButton::NoButton,
        );
    }

    unsafe fn drag(
        &self,
        window: *mut QQuickWindow,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
    ) {
        let Some(mut window) = (unsafe { pin_window(window) }) else {
            return;
        };
        let from = QPointF::new(from_x, from_y);
        // the pointer arrives before it presses, as a real one does
        send_mouse(
            window.as_mut(),
            QEventType::MouseMove,
            &from,
            MouseButton::NoButton,
        );
        send_mouse(
            window.as_mut(),
            QEventType::MouseButtonPress,
            &from,
            MouseButton::LeftButton,
        );
        const STEPS: i32 = 10;
        for i in 1..=STEPS {
            let t = f64::from(i) / f64::from(STEPS);
            let point = QPointF::new(from_x + (to_x - from_x) * t, from_y + (to_y - from_y) * t);
            send_mouse(
                window.as_mut(),
                QEventType::MouseMove,
                &point,
                MouseButton::LeftButton,
            );
        }
        let to = QPointF::new(to_x, to_y);
        send_mouse(
            window,
            QEventType::MouseButtonRelease,
            &to,
            MouseButton::NoButton,
        );
    }

    unsafe fn key(&self, window: *mut QQuickWindow, key: i32, modifiers: i32, text: &QString) {
        let Some(mut window) = (unsafe { pin_window(window) }) else {
            return;
        };
        let modifiers = KeyboardModifiers::from_int(modifiers as _);
        for event_type in [QEventType::KeyPress, QEventType::KeyRelease] {
            let mut event = qobject::qkeyevent_new(event_type, key, modifiers, text);
            // SAFETY: the window and the event live for the call, which delivers the event right
            // away
            unsafe {
                let receiver: *mut QObject =
                    Upcast::<QObject>::upcast_pin(window.as_mut()).get_unchecked_mut();
                let event: *mut QEvent =
                    Upcast::<QEvent>::upcast_pin(event.pin_mut()).get_unchecked_mut();
                qobject::QCoreApplication::send_event(receiver, event);
            }
        }
    }

    unsafe fn item_at(&self, window: *mut QQuickWindow, x: f64, y: f64) -> *mut QQuickItem {
        // SAFETY: QML passes a window that it holds, and the items of its scene live while it
        // is handled
        let root = unsafe {
            window
                .as_ref()
                .and_then(|window| window.content_item().as_ref())
        };
        let Some(mut root) = root else {
            return std::ptr::null_mut();
        };
        while let Some(parent) = unsafe { root.parent_item().as_ref() } {
            root = parent;
        }
        hit_test(root, &QPointF::new(x, y)).map_or(std::ptr::null_mut(), |item| {
            std::ptr::from_ref(item).cast_mut()
        })
    }

    unsafe fn set_blur_behind(&self, window: *mut QQuickWindow, enabled: bool) -> bool {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return false;
        };
        platform::set_blur_behind(Upcast::<QWindow>::upcast_pin(window), enabled)
    }

    unsafe fn set_dark_frame(&self, window: *mut QQuickWindow, dark: bool) -> bool {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return false;
        };
        platform::set_dark_frame(Upcast::<QWindow>::upcast_pin(window), dark)
    }

    unsafe fn hide_title_bar(&self, window: *mut QQuickWindow) -> bool {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return false;
        };
        platform::hide_title_bar(Upcast::<QWindow>::upcast_pin(window))
    }

    unsafe fn show_window_menu(&self, window: *mut QQuickWindow, x: f64, y: f64) -> bool {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return false;
        };
        platform::show_window_menu(Upcast::<QWindow>::upcast_pin(window), x, y)
    }

    unsafe fn tiled_edges(&self, window: *mut QQuickWindow) -> i32 {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return 0;
        };
        platform::tiled_edges(Upcast::<QWindow>::upcast_pin(window))
    }

    unsafe fn place_window_buttons(&self, window: *mut QQuickWindow, bar_height: f64) -> f64 {
        let Some(window) = (unsafe { pin_window(window) }) else {
            return 0.0;
        };
        platform::place_window_buttons(Upcast::<QWindow>::upcast_pin(window), bar_height)
    }

    unsafe fn hover_link(
        &self,
        edit: *mut QQuickItem,
        position: i32,
        href: &QString,
        link_color: &QColor,
        hover_color: &QColor,
    ) {
        // SAFETY: QML passes a live item or null
        if let Some(edit) = unsafe { edit.as_ref() } {
            qobject::hover_link(edit, position, href, link_color, hover_color);
        }
    }

    fn set_tab_handler(self: Pin<&mut Self>, handler: &QJSValue) {
        self.rust_mut().tab_handler = qobject::qjsvalue_clone(handler);
    }

    unsafe fn event_filter(
        mut self: Pin<&mut Self>,
        watched: *mut QObject,
        event: *mut QEvent,
    ) -> bool {
        // SAFETY: Qt passes the object and the event being delivered, which outlive the call
        let (Some(watched), Some(event)) = (unsafe { watched.as_ref() }, unsafe { event.as_ref() })
        else {
            return false;
        };
        let to_window = watched.downcast::<QQuickWindow>().is_some();
        match event.event_type() {
            // The press reaches the window first, then each item it is delivered to.
            QEventType::MouseButtonPress => {
                if let Some(mouse) = event.downcast::<QMouseEvent>().filter(|_| to_window) {
                    self.pointer_pressed(mouse.scene_position());
                }
            }
            QEventType::KeyPress => {
                let Some(key) = event.downcast::<QKeyEvent>() else {
                    return false;
                };
                let modifiers = key.modifiers();
                self.as_mut().set_modifiers(modifiers);
                if !to_window {
                    return false;
                }
                self.as_mut().key_pressed(key.key());
                let tab = key.key() == KEY_TAB || key.key() == KEY_BACKTAB;
                let other_modifiers = KeyboardModifier::ControlModifier
                    | KeyboardModifier::AltModifier
                    | KeyboardModifier::MetaModifier;
                if tab
                    && !modifiers.test_any_flags(other_modifiers)
                    && self.tab_handler.as_ref().is_some_and(QJSValue::is_callable)
                {
                    let forward = key.key() == KEY_TAB
                        && !modifiers.test_flag(KeyboardModifier::ShiftModifier);
                    // a copy, as the handler may replace itself
                    let handler = qobject::qjsvalue_clone(&self.tab_handler);
                    return qobject::call_with_bool(&handler, forward);
                }
            }
            QEventType::KeyRelease => {
                if let Some(key) = event.downcast::<QKeyEvent>() {
                    self.set_modifiers(key.modifiers());
                }
            }
            QEventType::Expose | QEventType::WindowStateChange if to_window => {
                self.window_configured();
            }
            // Key releases are not delivered while the app is in the background.
            QEventType::ApplicationStateChange => {
                self.set_modifiers(qobject::QGuiApplication::query_keyboard_modifiers());
            }
            _ => {}
        }
        false
    }

    fn set_modifiers(mut self: Pin<&mut Self>, modifiers: KeyboardModifiers) {
        if modifiers != self.modifiers {
            self.as_mut().rust_mut().modifiers = modifiers;
            self.modifiers_changed();
        }
    }
}

/// The window that QML passed to an invokable, unless it passed null.
///
/// # Safety
///
/// The pointer must be null or point to a live window, which QML holds for the call.
unsafe fn pin_window<'a>(window: *mut QQuickWindow) -> Option<Pin<&'a mut QQuickWindow>> {
    // SAFETY: a QObject never moves
    unsafe { window.as_mut().map(|window| Pin::new_unchecked(window)) }
}

fn send_mouse(
    window: Pin<&mut QQuickWindow>,
    event_type: QEventType,
    local: &QPointF,
    buttons: impl Into<MouseButtons>,
) {
    let global = Upcast::<QWindow>::upcast(&*window).map_to_global(local);
    let mut event = qobject::qmouseevent_new(
        event_type,
        local,
        local,
        &global,
        MouseButton::LeftButton,
        buttons.into(),
        qobject::QGuiApplication::keyboard_modifiers(),
    );
    // SAFETY: the window and the event live for the call, which delivers the event right away
    unsafe {
        let receiver: *mut QObject = Upcast::<QObject>::upcast_pin(window).get_unchecked_mut();
        let event: *mut QEvent = Upcast::<QEvent>::upcast_pin(event.pin_mut()).get_unchecked_mut();
        qobject::QCoreApplication::send_event(receiver, event);
    }
}

fn takes_input(item: &QQuickItem) -> bool {
    // The overlay of popups takes input while any are open, but only to pass it on or close
    // them; it is the popups, and their backdrops, that are pressed.
    // SAFETY: the class names are C strings
    if unsafe { item.inherits(c"QQuickOverlay".as_ptr()) } {
        return false;
    }
    if !item.accepted_mouse_buttons().is_empty() || item.accept_hover_events() {
        return true;
    }
    let children = item.children();
    (0..isize::from(children.size())).any(|i| {
        // SAFETY: the item's children live as long as it does
        unsafe { children.value(i.into()).as_ref() }
            .is_some_and(|child| unsafe { child.inherits(c"QQuickPointerHandler".as_ptr()) })
    })
}

fn hit_test<'a>(item: &'a QQuickItem, scene_pos: &QPointF) -> Option<&'a QQuickItem> {
    if !item.is_visible() {
        return None;
    }
    // Shadows reach past what casts them, but like CSS's box-shadow, they are never what the
    // pointer is over.
    // SAFETY: the class name is a C string
    if unsafe { item.inherits(c"QQuickRectangularShadow".as_ptr()) } {
        return None;
    }
    let inside = item.contains(&item.map_from_scene(scene_pos));
    if item.clip() && !inside {
        return None;
    }
    let list = qobject::child_items(item);
    let mut children: Vec<&QQuickItem> = (0..isize::from(list.size()))
        // SAFETY: the item's children live as long as it does
        .filter_map(|i| unsafe { list.value(i.into()).as_ref() })
        .collect();
    // later children, and those with a greater z, are drawn above earlier ones
    children.sort_by(|a, b| a.z().total_cmp(&b.z()));
    if let Some(found) = children
        .iter()
        .rev()
        .find_map(|child| hit_test(child, scene_pos))
    {
        return Some(found);
    }
    let draws = item.flags().test_flag(QQuickItemFlag::ItemHasContents);
    (inside && (draws || takes_input(item))).then_some(item)
}

#[cfg(target_vendor = "apple")]
use super::native_macos as platform;
#[cfg(all(unix, not(target_vendor = "apple")))]
use super::native_unix as platform;
#[cfg(windows)]
use super::native_win32 as platform;

#[cfg(not(any(windows, unix)))]
mod platform {
    use core::pin::Pin;

    use super::QWindow;

    pub fn set_blur_behind(_window: Pin<&mut QWindow>, _enabled: bool) -> bool {
        false
    }

    pub fn set_dark_frame(_window: Pin<&mut QWindow>, _dark: bool) -> bool {
        false
    }

    pub fn hide_title_bar(_window: Pin<&mut QWindow>) -> bool {
        false
    }

    pub fn show_window_menu(_window: Pin<&mut QWindow>, _x: f64, _y: f64) -> bool {
        false
    }

    pub fn tiled_edges(_window: Pin<&mut QWindow>) -> i32 {
        0
    }

    pub fn place_window_buttons(_window: Pin<&mut QWindow>, _bar_height: f64) -> f64 {
        0.0
    }
}
