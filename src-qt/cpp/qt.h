#ifndef MANDERROW_QT_H
#define MANDERROW_QT_H

// What the Rust bindings to Qt need but cxx can't express: names at namespace scope for the
// types nested in Qt's classes, and calls that pass a QList or a QJSValue by value.

#include <memory>

#include <QtCore/QEvent>
#include <QtCore/QList>
#include <QtGui/QClipboard>
#include <QtQml/QJSValue>
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

} // namespace manderrow

#endif // MANDERROW_QT_H
