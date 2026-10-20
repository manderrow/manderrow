#include "manderrow/cpp/native.h"

#include <algorithm>

#if defined(Q_OS_WIN)
#include <dwmapi.h>
#include <windows.h>
#elif defined(Q_OS_UNIX) && !defined(Q_OS_DARWIN)
#include <dlfcn.h>

#include <QtGui/qguiapplication_platform.h>
// a QPA header, which Qt installs with its private headers, for Wayland's xdg_toplevel
#if __has_include(<qpa/qplatformnativeinterface.h>)
#include <qpa/qplatformnativeinterface.h>
#define MANDERROW_HAS_QPA 1
#endif
#endif

#include <QtCore/QResource>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtGui/QDesktopServices>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QRegion>
#include <QtGui/QWindow>
#include <QtQml/QQmlDebuggingEnabler>
#include <QtQuick/QQuickWindow>
#include <QtQuickControls2/QQuickStyle>

Native::Native(QObject* parent)
    : QObject(parent)
    , m_modifiers(QGuiApplication::queryKeyboardModifiers())
{
    QGuiApplication::instance()->installEventFilter(this);
}

bool Native::shift() const
{
    return m_modifiers.testFlag(Qt::ShiftModifier);
}

bool Native::ctrl() const
{
    return m_modifiers.testFlag(Qt::ControlModifier);
}

void Native::copyText(const QString& text) const
{
    QGuiApplication::clipboard()->setText(text);
}

bool Native::openFolder(const QString& path) const
{
    return QDesktopServices::openUrl(QUrl::fromLocalFile(path));
}

bool Native::grabWindow(QObject* window, const QString& path) const
{
    auto* quickWindow = qobject_cast<QQuickWindow*>(window);
    return quickWindow != nullptr && quickWindow->grabWindow().save(path);
}

namespace {
void sendMouse(QQuickWindow* window, QEvent::Type type, QPointF local, Qt::MouseButtons buttons)
{
    QMouseEvent event(type, local, local, window->mapToGlobal(local), Qt::LeftButton, buttons,
                      QGuiApplication::keyboardModifiers());
    QCoreApplication::sendEvent(window, &event);
}
} // namespace

void Native::click(QObject* window, qreal x, qreal y) const
{
    auto* quickWindow = qobject_cast<QQuickWindow*>(window);
    if (quickWindow == nullptr)
        return;
    sendMouse(quickWindow, QEvent::MouseMove, { x, y }, Qt::NoButton);
    sendMouse(quickWindow, QEvent::MouseButtonPress, { x, y }, Qt::LeftButton);
    sendMouse(quickWindow, QEvent::MouseButtonRelease, { x, y }, Qt::NoButton);
}

void Native::drag(QObject* window, qreal fromX, qreal fromY, qreal toX, qreal toY) const
{
    auto* quickWindow = qobject_cast<QQuickWindow*>(window);
    if (quickWindow == nullptr)
        return;
    const QPointF from(fromX, fromY);
    const QPointF to(toX, toY);
    // the pointer arrives before it presses, as a real one does
    sendMouse(quickWindow, QEvent::MouseMove, from, Qt::NoButton);
    sendMouse(quickWindow, QEvent::MouseButtonPress, from, Qt::LeftButton);
    constexpr int steps = 10;
    for (int i = 1; i <= steps; ++i)
        sendMouse(quickWindow, QEvent::MouseMove, from + (to - from) * i / steps, Qt::LeftButton);
    sendMouse(quickWindow, QEvent::MouseButtonRelease, to, Qt::NoButton);
}

namespace {
bool takesInput(const QQuickItem* item)
{
    // The overlay of popups takes input while any are open, but only to pass it on or close
    // them; it is the popups, and their backdrops, that are pressed.
    if (item->inherits("QQuickOverlay"))
        return false;
    if (item->acceptedMouseButtons() != Qt::NoButton || item->acceptHoverEvents())
        return true;
    const auto& children = item->children();
    return std::any_of(children.begin(), children.end(), [](const QObject* child) {
        return child->inherits("QQuickPointerHandler");
    });
}

QQuickItem* hitTest(QQuickItem* item, QPointF scenePos)
{
    if (!item->isVisible())
        return nullptr;
    const bool inside = item->contains(item->mapFromScene(scenePos));
    if (item->clip() && !inside)
        return nullptr;
    // later children, and those with a greater z, are drawn above earlier ones
    QList<QQuickItem*> children = item->childItems();
    std::stable_sort(children.begin(), children.end(),
                     [](const QQuickItem* a, const QQuickItem* b) { return a->z() < b->z(); });
    for (auto it = children.crbegin(); it != children.crend(); ++it) {
        if (QQuickItem* found = hitTest(*it, scenePos))
            return found;
    }
    if (inside && (item->flags().testFlag(QQuickItem::ItemHasContents) || takesInput(item)))
        return item;
    return nullptr;
}
} // namespace

bool Native::setBlurBehind(QObject* window, bool enabled) const
{
    auto* qwindow = qobject_cast<QWindow*>(window);
    if (qwindow == nullptr)
        return false;
#if defined(Q_OS_WIN)
    const auto hwnd = reinterpret_cast<HWND>(qwindow->winId());
    // DWMWA_SYSTEMBACKDROP_TYPE, with DWMSBT_TRANSIENTWINDOW (acrylic) or DWMSBT_NONE
    constexpr DWORD systemBackdropType = 38;
    const int backdrop = enabled ? 3 : 1;
    // the backdrop shows through the client area where the frame is extended into it
    const MARGINS margins = enabled ? MARGINS { -1, -1, -1, -1 } : MARGINS { 0, 0, 0, 0 };
    DwmExtendFrameIntoClientArea(hwnd, &margins);
    return SUCCEEDED(DwmSetWindowAttribute(hwnd, systemBackdropType, &backdrop, sizeof backdrop));
#elif defined(Q_OS_UNIX) && !defined(Q_OS_DARWIN)
    // KWindowEffects::enableBlurBehind, loaded rather than linked so that the app doesn't depend
    // on KDE's libraries. It works on X11 and Wayland.
    using EnableBlurBehind = void (*)(QWindow*, bool, const QRegion&);
    static const EnableBlurBehind enableBlurBehind = []() -> EnableBlurBehind {
        void* library = dlopen("libKF6WindowSystem.so.6", RTLD_NOW | RTLD_LOCAL);
        if (library == nullptr)
            return nullptr;
        return reinterpret_cast<EnableBlurBehind>(
            dlsym(library, "_ZN14KWindowEffects16enableBlurBehindEP7QWindowbRK7QRegion"));
    }();
    if (enableBlurBehind == nullptr)
        return false;
    // an empty region is the whole window
    enableBlurBehind(qwindow, enabled, QRegion());
    return true;
#else
    Q_UNUSED(enabled);
    return false;
#endif
}

bool Native::showWindowMenu(QObject* window, qreal x, qreal y) const
{
    auto* qwindow = qobject_cast<QWindow*>(window);
    if (qwindow == nullptr)
        return false;
#if defined(Q_OS_WIN)
    const auto hwnd = reinterpret_cast<HWND>(qwindow->winId());
    const HMENU menu = GetSystemMenu(hwnd, FALSE);
    if (menu == nullptr)
        return false;
    // the menu's items follow the window's state, as they do for a native title bar
    const bool maximized = IsZoomed(hwnd);
    const auto enable = [&](UINT command, bool enabled) {
        EnableMenuItem(menu, command, MF_BYCOMMAND | (enabled ? MF_ENABLED : MF_GRAYED));
    };
    enable(SC_RESTORE, maximized);
    enable(SC_MOVE, !maximized);
    enable(SC_SIZE, !maximized);
    enable(SC_MAXIMIZE, !maximized);
    enable(SC_MINIMIZE, true);
    enable(SC_CLOSE, true);
    SetMenuDefaultItem(menu, SC_CLOSE, FALSE);
    POINT point { static_cast<LONG>(x * qwindow->devicePixelRatio()),
                  static_cast<LONG>(y * qwindow->devicePixelRatio()) };
    ClientToScreen(hwnd, &point);
    const UINT command = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd,
                                        nullptr);
    if (command != 0)
        PostMessageW(hwnd, WM_SYSCOMMAND, command, 0);
    return true;
#elif defined(MANDERROW_HAS_QPA)
    auto* wayland = qGuiApp->nativeInterface<QNativeInterface::QWaylandApplication>();
    if (wayland == nullptr)
        return false;
    void* toplevel = QGuiApplication::platformNativeInterface()->nativeResourceForWindow("xdg_toplevel", qwindow);
    if (toplevel == nullptr)
        return false;
    // xdg_toplevel_show_window_menu, from libwayland-client, which Qt has already loaded. It is
    // looked up rather than linked so that the app doesn't depend on it.
    using MarshalFlags = void* (*)(void*, std::uint32_t, const void*, std::uint32_t, std::uint32_t, ...);
    using GetVersion = std::uint32_t (*)(void*);
    static void* const library = dlopen("libwayland-client.so.0", RTLD_NOW | RTLD_NOLOAD);
    if (library == nullptr)
        return false;
    static const auto marshalFlags = reinterpret_cast<MarshalFlags>(dlsym(library, "wl_proxy_marshal_flags"));
    static const auto getVersion = reinterpret_cast<GetVersion>(dlsym(library, "wl_proxy_get_version"));
    if (marshalFlags == nullptr || getVersion == nullptr)
        return false;
    constexpr std::uint32_t showWindowMenuOpcode = 4;
    marshalFlags(toplevel, showWindowMenuOpcode, nullptr, getVersion(toplevel), 0, wayland->lastInputSeat(),
                 wayland->lastInputSerial(), static_cast<std::int32_t>(x), static_cast<std::int32_t>(y));
    return true;
#else
    Q_UNUSED(x);
    Q_UNUSED(y);
    return false;
#endif
}

void Native::setTabHandler(const QJSValue& handler)
{
    m_tabHandler = handler;
}

QQuickItem* Native::itemAt(QObject* window, qreal x, qreal y) const
{
    auto* quickWindow = qobject_cast<QQuickWindow*>(window);
    if (quickWindow == nullptr)
        return nullptr;
    QQuickItem* root = quickWindow->contentItem();
    while (root->parentItem() != nullptr)
        root = root->parentItem();
    return hitTest(root, { x, y });
}

bool Native::eventFilter(QObject* watched, QEvent* event)
{
    switch (event->type()) {
    case QEvent::MouseButtonPress:
        // The press reaches the window first, then each item it is delivered to.
        if (qobject_cast<QQuickWindow*>(watched) != nullptr)
            Q_EMIT pointerPressed(static_cast<QMouseEvent*>(event)->scenePosition());
        break;
    case QEvent::KeyPress: {
        auto* key = static_cast<QKeyEvent*>(event);
        setModifiers(key->modifiers());
        if (qobject_cast<QQuickWindow*>(watched) == nullptr)
            break;
        Q_EMIT keyPressed(key->key());
        const bool tab = key->key() == Qt::Key_Tab || key->key() == Qt::Key_Backtab;
        if (tab && m_tabHandler.isCallable()
            && !(key->modifiers() & (Qt::ControlModifier | Qt::AltModifier | Qt::MetaModifier))) {
            const bool forward = key->key() == Qt::Key_Tab && !(key->modifiers() & Qt::ShiftModifier);
            if (m_tabHandler.call({ QJSValue(forward) }).toBool())
                return true;
        }
        break;
    }
    case QEvent::KeyRelease:
        setModifiers(static_cast<QKeyEvent*>(event)->modifiers());
        break;
    case QEvent::ApplicationStateChange:
        // Key releases are not delivered while the app is in the background.
        setModifiers(QGuiApplication::queryKeyboardModifiers());
        break;
    default:
        break;
    }
    return QObject::eventFilter(watched, event);
}

void Native::setModifiers(Qt::KeyboardModifiers modifiers)
{
    if (modifiers != m_modifiers) {
        m_modifiers = modifiers;
        Q_EMIT modifiersChanged();
    }
}

bool manderrow_register_resources(rust::Slice<const std::uint8_t> data)
{
    return QResource::registerResource(reinterpret_cast<const uchar*>(data.data()));
}

void manderrow_configure_before_app()
{
    // Every control is styled by the app, so use the style that is designed to be customized.
    QQuickStyle::setStyle(QStringLiteral("Basic"));
    // windows can be made translucent, which Qt only allows for windows created with an alpha
    // channel
    QQuickWindow::setDefaultAlphaBuffer(true);
    QGuiApplication::setApplicationName(QStringLiteral("Manderrow"));
    QGuiApplication::setOrganizationName(QStringLiteral("Manderrow"));
    QGuiApplication::setDesktopFileName(QStringLiteral("io.github.manderrow"));
}

void manderrow_enable_qml_debugging()
{
    QQmlDebuggingEnabler::enableDebugging(true);
}

void manderrow_configure_app()
{
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/assets/icon.png")));
    for (const auto& font : { "Inter-Variable.ttf", "Inter-Italic-Variable.ttf",
                              "SourceCodePro-Variable.ttf", "SourceCodePro-Italic-Variable.ttf",
                              "ManderrowIcons.ttf" }) {
        QFontDatabase::addApplicationFont(QStringLiteral(":/assets/fonts/") + QLatin1StringView(font));
    }
}
