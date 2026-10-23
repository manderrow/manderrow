#ifndef MANDERROW_UNIX_H
#define MANDERROW_UNIX_H

// What the Rust bindings to Qt's Wayland platform need but cxx can't express: a template, around a
// protected function, and Qt's private platform interface, which returns void pointers and is only
// available when Qt's private headers are installed.

#include <dlfcn.h>

#include <QtCore/QMetaObject>
#include <QtGui/QGuiApplication>
#include <QtGui/QWindow>
#include <QtGui/qguiapplication_platform.h>
#if __has_include(<qpa/qplatformnativeinterface.h>)
#include <qpa/qplatformnativeinterface.h>
#include <qpa/qplatformwindow.h>
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

/// The edges of a window that its Wayland compositor has tiled, from the tiled_left, tiled_right,
/// tiled_top, and tiled_bottom states of its xdg_toplevel: bits 1, 2, 4, and 8. Qt's Wayland
/// platform keeps them, but only offers them through its private API, which is looked up at run
/// time rather than linked to, as the platform is a plugin that may not even be installed. It is 0
/// when the window isn't on Wayland, or the API or Qt's private headers weren't available.
inline int waylandTiledEdges(const QWindow& window)
{
#if defined(MANDERROW_HAS_QPA)
    // QFlags<ToplevelWindowTilingState> is an int, which returns as one
    using TilingStates = int (*)(const void* self);
    static TilingStates tilingStates = nullptr;
    static bool resolved = false;

    // QtWaylandClient::QWaylandWindow derives from a QObject first and QPlatformWindow second, so
    // the QObject's address is the window's
    const QObject* platformWindow = dynamic_cast<const QObject*>(window.handle());
    if (!platformWindow)
        return 0;
    const QMetaObject* meta = platformWindow->metaObject();
    while (meta && qstrcmp(meta->className(), "QtWaylandClient::QWaylandWindow") != 0)
        meta = meta->superClass();
    if (!meta)
        return 0;
    if (!resolved) {
        resolved = true;
        // the library that defines QWaylandWindow, which Qt has already loaded
        Dl_info info;
        if (dladdr(meta, &info) && info.dli_fname) {
            if (void* library = dlopen(info.dli_fname, RTLD_LAZY | RTLD_NOLOAD)) {
                tilingStates = reinterpret_cast<TilingStates>(dlsym(
                    library, "_ZNK15QtWaylandClient14QWaylandWindow26toplevelWindowTilingStatesEv"));
                // which stays loaded, as Qt holds it too
                dlclose(library);
            }
        }
    }
    return tilingStates ? tilingStates(platformWindow) & 0xF : 0;
#else
    Q_UNUSED(window);
    return 0;
#endif
}

} // namespace manderrow

#endif // MANDERROW_UNIX_H
