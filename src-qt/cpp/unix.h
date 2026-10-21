#ifndef MANDERROW_UNIX_H
#define MANDERROW_UNIX_H

// What the Rust bindings to Qt's Wayland platform need but cxx can't express: a template, around a
// protected function, and Qt's private platform interface, which returns void pointers and is only
// available when Qt's private headers are installed.

#include <QtGui/QGuiApplication>
#include <QtGui/QWindow>
#include <QtGui/qguiapplication_platform.h>
#if __has_include(<qpa/qplatformnativeinterface.h>)
#include <qpa/qplatformnativeinterface.h>
#define MANDERROW_HAS_QPA 1
#endif

struct wl_proxy;

namespace manderrow {

/// The application's Wayland connection, or null when it isn't running on Wayland.
inline QNativeInterface::QWaylandApplication* waylandApplication()
{
    return qGuiApp->nativeInterface<QNativeInterface::QWaylandApplication>();
}

/// One of a window's Wayland objects, such as its "surface" or "xdg_toplevel", or null when it has
/// none or Qt's private headers weren't available to the build.
inline wl_proxy* waylandWindowResource(QWindow& window, const char* name)
{
#if defined(MANDERROW_HAS_QPA)
    return static_cast<wl_proxy*>(
        QGuiApplication::platformNativeInterface()->nativeResourceForWindow(name, &window));
#else
    Q_UNUSED(window);
    Q_UNUSED(name);
    return nullptr;
#endif
}

} // namespace manderrow

#endif // MANDERROW_UNIX_H
