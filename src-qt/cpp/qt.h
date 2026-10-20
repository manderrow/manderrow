#ifndef MANDERROW_QT_H
#define MANDERROW_QT_H

// What the Rust bindings to Qt need but cxx can't express: names at namespace scope for the
// types nested in Qt's classes, and calls that pass a QList or a QJSValue by value.

#include <memory>

#include <QtCore/QEvent>
#include <QtCore/QList>
#include <QtCore/QStringList>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtQml/QJSValue>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlError>
#include <QtQuick/QQuickItem>

namespace manderrow {

using QEventType = QEvent::Type;
using QClipboardMode = QClipboard::Mode;
using QQuickItemFlag = QQuickItem::Flag;
using QQuickItemFlags = QQuickItem::Flags;
using QQuickItemList = QList<QQuickItem*>;

inline std::unique_ptr<QQuickItemList> childItems(const QQuickItem& item)
{
    return std::make_unique<QQuickItemList>(item.childItems());
}

/// Calls a function with a boolean, and returns its result as a boolean.
inline bool callWithBool(const QJSValue& function, bool argument)
{
    return function.call({ QJSValue(argument) }).toBool();
}

/// Loads a local QML file, which loads synchronously, and returns the errors that kept it from
/// creating its root object, or an empty string if it created one.
inline QString loadRoot(QQmlApplicationEngine& engine, const QUrl& url)
{
    QStringList errors;
    const auto connection = QObject::connect(
        &engine, &QQmlEngine::warnings, [&](const QList<QQmlError>& warnings) {
            for (const QQmlError& warning : warnings)
                errors << warning.toString();
        });
    engine.load(url);
    QObject::disconnect(connection);
    if (!engine.rootObjects().isEmpty())
        return {};
    return errors.isEmpty() ? QStringLiteral("No error was reported") : errors.join(u'\n');
}

} // namespace manderrow

#endif // MANDERROW_QT_H
