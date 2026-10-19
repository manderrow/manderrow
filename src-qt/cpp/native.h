#ifndef MANDERROW_NATIVE_H
#define MANDERROW_NATIVE_H

#include <cstdint>

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtQml/QQmlEngine>

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

    /// Saves a screenshot of a window, including its popups. Returns whether it succeeded.
    Q_INVOKABLE bool grabWindow(QObject* window, const QString& path) const;

    bool eventFilter(QObject* watched, QEvent* event) override;

Q_SIGNALS:
    void modifiersChanged();

private:
    void setModifiers(Qt::KeyboardModifiers modifiers);

    Qt::KeyboardModifiers m_modifiers;
};

/// Registers a binary resource bundle produced by `rcc --binary`. The data must outlive the app.
bool manderrow_register_resources(rust::Slice<const std::uint8_t> data);

/// Configuration that must happen before the QGuiApplication is created.
void manderrow_configure_before_app();

/// Configuration that must happen after the QGuiApplication is created.
void manderrow_configure_app();

#endif // MANDERROW_NATIVE_H
