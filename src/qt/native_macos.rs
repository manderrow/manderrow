//! Native's window services on macOS, through AppKit, on the NSWindow that holds a QWindow's view.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qtypes.h");
        type quintptr = cxx_qt_lib::quintptr;

        include!(<QtGui/QWindow>);
        type QWindow = crate::qt::native::qobject::QWindow;

        #[rust_name = "win_id"]
        fn winId(self: &QWindow) -> quintptr;
    }
}

use core::cell::{Cell, RefCell};
use core::pin::Pin;
use core::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSButton, NSColor, NSTitlebarSeparatorStyle, NSView, NSWindow, NSWindowButton,
    NSWindowDidBecomeKeyNotification, NSWindowDidExitFullScreenNotification,
    NSWindowDidResizeNotification, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{NSNotification, NSNotificationCenter};

use ffi::QWindow;

/// The NSWindow that holds a QWindow, which Qt makes along with the QWindow's platform window.
fn ns_window(window: &QWindow) -> Option<Retained<NSWindow>> {
    // which creates the platform window if the QWindow has none yet
    let view = usize::from(window.win_id()) as *mut NSView;
    // SAFETY: a QWindow's id is its NSView on macOS, which lives as long as the QWindow
    unsafe { Retained::retain(view) }?.window()
}

/// How far the blur behind a window spreads, in points.
const BLUR_RADIUS: usize = 40;

// The window server's own blur behind a window, which, as Windows' acrylic accent, is private but
// has been there for years, and blurs what is behind the window's transparent parts without a tint
// of its own, leaving the app's, as on Linux and Windows. iTerm2, Alacritty, and kitty blur their
// windows with it. AppKit's NSVisualEffectView would add a material of its own, and only blurs
// behind views that are below the window's content view, which Qt draws into.
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// The app's connection to the window server, a CGSConnectionID
    fn CGSMainConnectionID() -> i32;
    /// Returns a CGError, which is 0 for success.
    fn CGSSetWindowBackgroundBlurRadius(connection: i32, window: u32, radius: usize) -> i32;
}

pub fn set_blur_behind(window: Pin<&mut QWindow>, enabled: bool) -> bool {
    let Some(ns_window) = ns_window(&window) else {
        return false;
    };
    // the window server only has the window once it has been shown
    let Some(number) = u32::try_from(ns_window.windowNumber())
        .ok()
        .filter(|&number| number != 0)
    else {
        return false;
    };
    let radius = if enabled { BLUR_RADIUS } else { 0 };
    // SAFETY: the functions have these signatures, and the window is one of the app's
    unsafe { CGSSetWindowBackgroundBlurRadius(CGSMainConnectionID(), number, radius) == 0 }
}

pub fn set_dark_frame(window: Pin<&mut QWindow>, dark: bool) -> bool {
    let Some(ns_window) = ns_window(&window) else {
        return false;
    };
    // SAFETY: the names are AppKit's constants
    let name = unsafe {
        if dark {
            NSAppearanceNameDarkAqua
        } else {
            NSAppearanceNameAqua
        }
    };
    let Some(appearance) = NSAppearance::appearanceNamed(name) else {
        return false;
    };
    ns_window.setAppearance(Some(&appearance));
    true
}

pub fn hide_title_bar(window: Pin<&mut QWindow>) -> bool {
    let Some(ns_window) = ns_window(&window) else {
        return false;
    };
    // The window's content already reaches under its title bar, which has no background, through
    // its flags. What is left of the title bar is its title, and its buttons, which the app's
    // title bar places with place_window_buttons.
    ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    ns_window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
    // Qt only clears the background of frameless windows, and fills that of others with the
    // system's, which would show through where the app is translucent instead of what is behind
    // the window.
    ns_window.setBackgroundColor(Some(&NSColor::clearColor()));
    true
}

pub fn show_window_menu(_window: Pin<&mut QWindow>, _x: f64, _y: f64) -> bool {
    // macOS title bars have no window menu
    false
}

pub fn tiled_edges(_window: Pin<&mut QWindow>) -> i32 {
    // Tiled windows aren't told that they are tiled
    0
}

/// A window whose buttons place_window_buttons keeps in place.
struct PlacedButtons {
    window: Weak<NSWindow>,
    bar_height: Rc<Cell<f64>>,
    observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl Drop for PlacedButtons {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: the observer is one that the center returned
            unsafe { center.removeObserver(observer.as_ref()) };
        }
    }
}

thread_local! {
    static PLACED_BUTTONS: RefCell<Vec<PlacedButtons>> = const { RefCell::new(Vec::new()) };
}

pub fn place_window_buttons(window: Pin<&mut QWindow>, bar_height: f64) -> f64 {
    let Some(ns_window) = ns_window(&window) else {
        return 0.0;
    };
    // AppKit lays the buttons out again as the window changes, as when it is resized or leaves
    // full screen, so they are placed again each time it does.
    PLACED_BUTTONS.with_borrow_mut(|placed| {
        // the windows that are gone
        placed.retain(|placed| placed.window.load().is_some());
        if let Some(placed) = placed.iter().find(|placed| {
            placed
                .window
                .load()
                .is_some_and(|w| Retained::as_ptr(&w) == Retained::as_ptr(&ns_window))
        }) {
            placed.bar_height.set(bar_height);
            return;
        }
        let height = Rc::new(Cell::new(bar_height));
        let weak = Weak::from_retained(&ns_window);
        let block = RcBlock::new({
            let height = height.clone();
            move |_: NonNull<NSNotification>| {
                if let Some(ns_window) = weak.load() {
                    place_buttons(&ns_window, height.get());
                }
            }
        });
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the names are AppKit's constants
        let names = unsafe {
            [
                NSWindowDidResizeNotification,
                NSWindowDidExitFullScreenNotification,
                NSWindowDidBecomeKeyNotification,
            ]
        };
        let observers = names
            .into_iter()
            .map(|name| {
                // SAFETY: the object is the window, and without a queue the block is called on the
                // thread that posts the notification, which for a window's is the main thread,
                // that the block was made on
                unsafe {
                    center.addObserverForName_object_queue_usingBlock(
                        Some(name),
                        Some(&ns_window),
                        None,
                        &block,
                    )
                }
            })
            .collect();
        placed.push(PlacedButtons {
            window: Weak::from_retained(&ns_window),
            bar_height: height,
            observers,
        });
    });
    place_buttons(&ns_window, bar_height)
}

/// Centers the window's buttons in a title bar of the app's height at the top of the window, as
/// Electron and Tauri place them. Returns how much of the bar they take from its left edge, with
/// twice the space between them after them, or 0 if it shows none, as in full screen.
fn place_buttons(ns_window: &NSWindow, bar_height: f64) -> f64 {
    // in full screen, the buttons are in a title bar that slides down over the window
    if ns_window
        .styleMask()
        .contains(NSWindowStyleMask::FullScreen)
    {
        return 0.0;
    }
    let buttons: Vec<Retained<NSButton>> = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .into_iter()
    .filter_map(|button| ns_window.standardWindowButton(button))
    .filter(|button| !button.isHidden())
    .collect();
    let Some(first) = buttons.first() else {
        return 0.0;
    };
    // The buttons are in the title bar's view, in a container at the top of the window, which is
    // sized to the app's title bar and fills it.
    // SAFETY: the views are AppKit's, which the window holds
    let Some(container) = (unsafe { first.superview().and_then(|bar| bar.superview()) }) else {
        return 0.0;
    };
    let mut frame = container.frame();
    frame.size.height = bar_height;
    frame.origin.y = ns_window.frame().size.height - bar_height;
    container.setFrame(frame);
    let mut right: f64 = 0.0;
    for button in &buttons {
        let mut frame = button.frame();
        frame.origin.y = ((bar_height - frame.size.height) / 2.0).round();
        button.setFrameOrigin(frame.origin);
        right = right.max(frame.origin.x + frame.size.width);
    }
    // the space between the first two buttons, or, with only one, the space before it
    let space = match &buttons[..] {
        [first, second, ..] => {
            let first = first.frame();
            second.frame().origin.x - (first.origin.x + first.size.width)
        }
        _ => first.frame().origin.x,
    };
    right + 2.0 * space
}
