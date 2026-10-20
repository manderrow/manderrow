#include "manderrow/cpp/native.h"

#include <QtCore/QResource>
#include <QtGui/QClipboard>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
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

bool Native::eventFilter(QObject* watched, QEvent* event)
{
    switch (event->type()) {
    case QEvent::MouseButtonPress:
        // The press reaches the window first, then each item it is delivered to.
        if (qobject_cast<QQuickWindow*>(watched) != nullptr)
            Q_EMIT pointerPressed(static_cast<QMouseEvent*>(event)->scenePosition());
        break;
    case QEvent::KeyPress:
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
                              "SourceCodePro-Variable.ttf", "SourceCodePro-Italic-Variable.ttf" }) {
        QFontDatabase::addApplicationFont(QStringLiteral(":/assets/fonts/") + QLatin1StringView(font));
    }
}
