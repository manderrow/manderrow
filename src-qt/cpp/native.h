#ifndef MANDERROW_NATIVE_H
#define MANDERROW_NATIVE_H

#include <cstdint>

#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtQml/QJSValue>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickItem>

#include "rust/cxx.h"

/// Platform services that QML can't provide on its own.
class Native : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    /// Whether the Shift key is currently held down, anywhere in the application.
    Q_PROPERTY(bool shift READ shift NOTIFY modifiersChanged)
    /// Whether the Control key is currently held down, anywhere in the application.
    Q_PROPERTY(bool ctrl READ ctrl NOTIFY modifiersChanged)

public:
    explicit Native(QObject* parent = nullptr);

    bool shift() const;
    bool ctrl() const;

    Q_INVOKABLE void copyText(const QString& text) const;
    /// Opens a folder in the platform's file manager. Returns whether it could.
    Q_INVOKABLE bool openFolder(const QString& path) const;

    /// Saves a screenshot of a window, including its popups. Returns whether it succeeded.
    Q_INVOKABLE bool grabWindow(QObject* window, const QString& path) const;

    /// Clicks a window at a point in its coordinates, as the mouse would. For development scripts.
    Q_INVOKABLE void click(QObject* window, qreal x, qreal y) const;
    /// Drags the mouse across a window, between points in its coordinates. For development scripts.
    Q_INVOKABLE void drag(QObject* window, qreal fromX, qreal fromY, qreal toX, qreal toY) const;
    /// The topmost item at a point of a window, in scene coordinates, that is drawn or takes
    /// input, as a browser finds the element under the pointer. Items that only arrange or hold
    /// others, like layouts and Loaders, don't count.
    Q_INVOKABLE QQuickItem* itemAt(QObject* window, qreal x, qreal y) const;
    /// Blurs what is behind a translucent window, where the platform can: on KDE Plasma, through
    /// KWindowSystem when it is installed, and on Windows 11, with the acrylic backdrop. Returns
    /// whether the platform can.
    Q_INVOKABLE bool setBlurBehind(QObject* window, bool enabled) const;
    /// Shows the window manager's menu for a window, as right clicking a native title bar does,
    /// at a point in the window's coordinates. Call it when the right button is pressed. Returns
    /// whether the platform can: on Wayland, when Qt's private platform headers were available
    /// to the build, and on Windows.
    Q_INVOKABLE bool showWindowMenu(QObject* window, qreal x, qreal y) const;
    /// Takes over moving the focus with Tab and Shift+Tab in every window: `handler(forward)` is
    /// called before the key reaches any item, and the key goes no further if it returns true.
    Q_INVOKABLE void setTabHandler(const QJSValue& handler);

    bool eventFilter(QObject* watched, QEvent* event) override;

Q_SIGNALS:
    void modifiersChanged();
    /// A mouse button was pressed in a window, at a point in its coordinates. This is emitted
    /// before the press is delivered to the window's items.
    void pointerPressed(const QPointF& position);
    /// A key was pressed in a window, before it is delivered to the focused item.
    void keyPressed(int key);

private:
    void setModifiers(Qt::KeyboardModifiers modifiers);

    QJSValue m_tabHandler;

    Qt::KeyboardModifiers m_modifiers;
};

/// Registers a binary resource bundle produced by `rcc --binary`. The data must outlive the app.
bool manderrow_register_resources(rust::Slice<const std::uint8_t> data);

/// Configuration that must happen before the QGuiApplication is created.
void manderrow_configure_before_app();
/// Lets QML tools, like qmlprofiler, attach. For development builds.
void manderrow_enable_qml_debugging();

/// Configuration that must happen after the QGuiApplication is created.
void manderrow_configure_app();

#endif // MANDERROW_NATIVE_H
