#include "manderrow/cpp/native.h"

#include <QtCore/QResource>
#include <QtGui/QClipboard>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QKeyEvent>
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

bool Native::eventFilter(QObject* watched, QEvent* event)
{
    switch (event->type()) {
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

void manderrow_configure_app()
{
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/assets/icon.png")));
    for (const auto& font : { "Inter-Variable.ttf", "Inter-Italic-Variable.ttf",
                              "SourceCodePro-Variable.ttf", "SourceCodePro-Italic-Variable.ttf" }) {
        QFontDatabase::addApplicationFont(QStringLiteral(":/assets/fonts/") + QLatin1StringView(font));
    }
}
