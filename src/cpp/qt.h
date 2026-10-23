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
#include <QtGui/QColor>
#include <QtGui/QTextBlock>
#include <QtGui/QTextCursor>
#include <QtGui/QTextDocument>
#include <QtQml/QJSValue>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlError>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickTextDocument>

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

namespace detail {

/// The properties of a hovered link's character formats: whether it is hovered, and what it had
/// before for the properties that hovering changes.
enum HoveredLinkProperty : int {
    Hovered = QTextFormat::UserProperty + 1,
    UnhoveredUnderline,
    UnhoveredForeground,
};

/// A range of a document's text, and the character format to give it.
struct FormatRange {
    int start;
    int end;
    QTextCharFormat format;
};

inline void setFormats(QTextDocument& document, const QList<FormatRange>& ranges)
{
    QTextCursor cursor(&document);
    for (const FormatRange& range : ranges) {
        cursor.setPosition(range.start);
        cursor.setPosition(range.end, QTextCursor::KeepAnchor);
        cursor.setCharFormat(range.format);
    }
}

} // namespace detail

/// Shows the link at a position in a rich text TextEdit as hovered, as a browser styles
/// `a:hover`: underlined, and in `hoverColor` where it is in `linkColor`, so that what has a
/// color of its own, like code, keeps it. The link shown as hovered before in it goes back to how
/// it was. An empty `href` shows none as hovered.
inline void hoverLink(const QQuickItem& edit, int position, const QString& href,
    const QColor& linkColor, const QColor& hoverColor)
{
    auto* quickDocument = qobject_cast<QQuickTextDocument*>(
        edit.property("textDocument").value<QObject*>());
    if (!quickDocument)
        return;
    QTextDocument& document = *quickDocument->textDocument();
    // the formats are not edits to undo
    document.setUndoRedoEnabled(false);

    // the fragments of the link with the href around the position
    QList<QTextFragment> link;
    if (!href.isEmpty()) {
        const auto covers = [position](const QList<QTextFragment>& run) {
            return !run.isEmpty() && run.first().position() <= position
                && position <= run.last().position() + run.last().length();
        };
        const QTextBlock block = document.findBlock(position);
        for (auto it = block.begin(); !it.atEnd(); ++it) {
            const QTextFragment fragment = it.fragment();
            const QTextCharFormat format = fragment.charFormat();
            if (format.isAnchor() && format.anchorHref() == href) {
                link.append(fragment);
            } else if (covers(link)) {
                break;
            } else {
                link.clear();
            }
        }
        if (!covers(link))
            link.clear();
        if (!link.isEmpty() && link.first().charFormat().hasProperty(detail::Hovered))
            return;
    }

    QList<detail::FormatRange> hovered;
    for (const QTextFragment& fragment : link) {
        const QTextCharFormat format = fragment.charFormat();
        QTextCharFormat hover = format;
        hover.setProperty(detail::Hovered, true);
        hover.setProperty(detail::UnhoveredUnderline, format.property(QTextFormat::TextUnderlineStyle));
        hover.setProperty(detail::UnhoveredForeground, format.property(QTextFormat::ForegroundBrush));
        hover.setFontUnderline(true);
        if (format.foreground().style() != Qt::NoBrush
            && format.foreground().color().rgb() == linkColor.rgb())
            hover.setForeground(hoverColor);
        hovered.append({ fragment.position(), fragment.position() + fragment.length(), hover });
    }

    QList<detail::FormatRange> unhovered;
    for (QTextBlock block = document.begin(); block.isValid(); block = block.next()) {
        for (auto it = block.begin(); !it.atEnd(); ++it) {
            const QTextFragment fragment = it.fragment();
            QTextCharFormat format = fragment.charFormat();
            if (!format.hasProperty(detail::Hovered))
                continue;
            // an invalid value, which it had when it lacked the property, removes it
            format.setProperty(QTextFormat::TextUnderlineStyle, format.property(detail::UnhoveredUnderline));
            format.setProperty(QTextFormat::ForegroundBrush, format.property(detail::UnhoveredForeground));
            format.clearProperty(detail::Hovered);
            format.clearProperty(detail::UnhoveredUnderline);
            format.clearProperty(detail::UnhoveredForeground);
            unhovered.append({ fragment.position(), fragment.position() + fragment.length(), format });
        }
    }

    detail::setFormats(document, unhovered);
    detail::setFormats(document, hovered);
}

} // namespace manderrow

#endif // MANDERROW_QT_H
