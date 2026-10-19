import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// Lays out blocks produced by the backend's Markdown renderer, recursively.
Column {
    id: root

    property var blocks: []
    /// Whether these are the document's top-level blocks, whose first heading has no top margin.
    property bool first: false
    /// Paragraphs in tight lists and blockquotes have no margins.
    property bool compact: false
    property color textColor: Theme.neutral05

    // .markdown's font size depends on the window width
    readonly property real baseSize: Theme.windowWidth < 32 * 16 ? 12 : Theme.windowWidth < 48 * 16 ? 15 : 16
    readonly property real rem: Theme.rem
    readonly property var headingSizes: [2, 1.75, 1.5, 1.25, 0.95, 0.88]

    function headingSize(level) {
        return headingSizes[level - 1] * rem;
    }

    function margins(block, index) {
        switch (block.type) {
        case "paragraph":
        case "html":
        case "image":
            return { top: 0, bottom: compact ? 0 : 0.75 * rem };
        case "heading":
            {
                const size = headingSize(block.level);
                if (block.level > 4)
                    return { top: 0, bottom: 0 };
                const top = block.level === 1 ? (first && index === 0 ? 0 : 0.5 * size) : 0.75 * size;
                return { top, bottom: 0.35 * size };
            }
        case "quote":
            return { top: 0.5 * baseSize, bottom: 0.5 * baseSize };
        case "list":
            return { top: 0, bottom: block.ordered ? 0 : baseSize };
        case "rule":
            return { top: baseSize, bottom: baseSize };
        default:
            return { top: 0, bottom: 0 };
        }
    }

    /// Wraps inline HTML with the styles rich text supports.
    function richText(html, lineHeight, size) {
        return `<style>a { color: ${Theme.primary60}; text-decoration: none; } code { font-family: "${Theme.fontMonospace}"; background-color: ${Theme.neutral90}; color: ${Theme.neutral05}; font-size: ${Math.round(0.9 * rem)}px; } del { text-decoration: line-through; }</style>` + `<div style="line-height: ${Math.round(lineHeight * size)}px">${html}</div>`;
    }

    component RichBlock: TextEdit {
        id: rich
        property string html
        property real size: root.rem
        property real lineHeight: 1.35

        width: parent ? parent.width : 0
        textFormat: TextEdit.RichText
        wrapMode: TextEdit.Wrap
        readOnly: true
        selectByMouse: true
        selectionColor: Theme.primary50
        color: root.textColor
        font.family: Theme.font
        font.pixelSize: size
        text: root.richText(html, lineHeight, size)
        onLinkActivated: link => Util.openUrl(link)

        HoverHandler {
            cursorShape: rich.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.IBeamCursor
        }
    }

    Repeater {
        model: root.blocks

        delegate: Item {
            id: slot

            required property var modelData
            required property int index
            readonly property var block: modelData
            readonly property var margin: root.margins(modelData, index)
            readonly property var previous: index > 0 ? root.margins(root.blocks[index - 1], index - 1) : ({ bottom: 0 })
            // adjacent vertical margins collapse, as in CSS
            readonly property real topGap: Math.max(0, margin.top - previous.bottom)

            width: root.width
            height: topGap + (loader.item ? loader.item.height : 0) + margin.bottom

            Loader {
                id: loader
                y: slot.topGap
                width: parent.width
                sourceComponent: {
                    switch (slot.block.type) {
                    case "heading":
                        return heading;
                    case "paragraph":
                        return paragraph;
                    case "html":
                        return paragraph;
                    case "image":
                        return image;
                    case "code":
                        return code;
                    case "quote":
                        return quote;
                    case "alert":
                        return alert;
                    case "list":
                        return list;
                    case "table":
                        return table;
                    case "rule":
                        return rule;
                    default:
                        return null;
                    }
                }

                Component {
                    id: heading
                    Item {
                        width: parent.width
                        height: headingText.height + (slot.block.level === 1 ? 1 : 0)

                        RichBlock {
                            id: headingText
                            html: slot.block.html
                            size: root.headingSize(slot.block.level)
                            lineHeight: 1.25
                        }
                        // h1 { border-bottom: 1px solid neutral-80 }
                        Rectangle {
                            visible: slot.block.level === 1
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: 1
                            color: Theme.neutral80
                        }
                    }
                }

                Component {
                    id: paragraph
                    RichBlock {
                        html: slot.block.html
                    }
                }

                Component {
                    id: image
                    Item {
                        width: parent.width
                        height: img.status === Image.Ready ? img.height : 0

                        Image {
                            id: img
                            source: slot.block.src
                            asynchronous: true
                            fillMode: Image.PreserveAspectFit
                            // img { max-width: 100% }
                            readonly property real naturalWidth: slot.block.width ? slot.block.width : implicitWidth
                            width: Math.min(naturalWidth, parent.width)
                            height: implicitWidth > 0 ? width * implicitHeight / implicitWidth : 0
                            x: slot.block.centered ? (parent.width - width) / 2 : 0

                            HoverHandler {
                                cursorShape: slot.block.href ? Qt.PointingHandCursor : Qt.ArrowCursor
                            }
                            TapHandler {
                                enabled: !!slot.block.href
                                onTapped: Util.openUrl(slot.block.href)
                            }
                        }
                    }
                }

                Component {
                    id: code
                    Rectangle {
                        width: parent.width
                        height: codeText.height + 2 * 0.5 * root.rem
                        color: Theme.neutral90

                        Flickable {
                            anchors.fill: parent
                            anchors.margins: 0.5 * root.rem
                            contentWidth: codeText.width
                            contentHeight: codeText.height
                            flickableDirection: Flickable.HorizontalFlick
                            acceptedButtons: Qt.NoButton
                            clip: true
                            ScrollBar.horizontal: AppScrollBar {}

                            TextEdit {
                                id: codeText
                                text: slot.block.text
                                textFormat: TextEdit.PlainText
                                readOnly: true
                                selectByMouse: true
                                selectionColor: Theme.primary50
                                color: Theme.neutral05
                                font.family: Theme.fontMonospace
                                font.pixelSize: 0.9 * root.rem
                            }
                        }
                    }
                }

                Component {
                    id: quote
                    Item {
                        width: parent.width
                        height: quoteBlocks.height + 2 * 0.5 * root.baseSize

                        Rectangle {
                            width: 4
                            height: parent.height
                            color: Theme.neutral40
                        }
                        Loader {
                            id: quoteBlocks
                            x: 4 + root.baseSize
                            y: 0.5 * root.baseSize
                            width: parent.width - x - root.baseSize
                            source: "MarkdownBlocks.qml"
                            onLoaded: {
                                item.blocks = Qt.binding(() => slot.block.children);
                                item.compact = true;
                                item.textColor = Theme.neutral20;
                                item.width = Qt.binding(() => quoteBlocks.width);
                            }
                        }
                    }
                }

                Component {
                    id: alert
                    Column {
                        width: parent.width

                        Row {
                            spacing: 0.5 * root.rem
                            bottomPadding: 0.75 * root.rem

                            FaIcon {
                                name: "octicon:" + slot.block.kind
                                size: 16
                                color: root.textColor
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            Text {
                                text: slot.block.title
                                color: root.textColor
                                font.family: Theme.font
                                font.pixelSize: root.rem
                            }
                        }
                        Loader {
                            width: parent.width
                            source: "MarkdownBlocks.qml"
                            onLoaded: {
                                item.blocks = Qt.binding(() => slot.block.children);
                                item.width = Qt.binding(() => width);
                            }
                        }
                    }
                }

                Component {
                    id: list
                    Column {
                        width: parent.width

                        Repeater {
                            model: slot.block.items

                            delegate: Item {
                                id: listItem

                                required property var modelData
                                required property int index

                                width: parent.width
                                // li { margin-left: 1.5em; margin-bottom: 0.25em }
                                height: itemBlocks.height + 0.25 * root.baseSize

                                Text {
                                    x: 1.5 * root.baseSize - width - 0.4 * root.baseSize
                                    visible: listItem.modelData.checked === null
                                    text: slot.block.ordered ? `${slot.block.start + listItem.index}.` : "•"
                                    color: root.textColor
                                    font.family: Theme.font
                                    font.pixelSize: root.baseSize
                                    lineHeight: 1.35 * root.rem
                                    lineHeightMode: Text.FixedHeight
                                }
                                Checkbox {
                                    visible: listItem.modelData.checked !== null
                                    x: 1.5 * root.baseSize - width - 0.4 * root.baseSize
                                    y: (1.35 * root.rem - height) / 2
                                    checked: listItem.modelData.checked === true
                                    size: 0.8 * root.baseSize
                                    enabled: false
                                }

                                Loader {
                                    id: itemBlocks
                                    x: 1.5 * root.baseSize
                                    width: parent.width - x
                                    source: "MarkdownBlocks.qml"
                                    onLoaded: {
                                        item.blocks = Qt.binding(() => listItem.modelData.children);
                                        item.compact = slot.block.tight;
                                        item.width = Qt.binding(() => itemBlocks.width);
                                    }
                                }
                            }
                        }
                    }
                }

                Component {
                    id: table
                    // Qt's rich text sizes table columns itself, so the table is rendered as HTML
                    Flickable {
                        width: parent.width
                        height: tableText.height
                        contentWidth: tableText.width
                        contentHeight: tableText.height
                        flickableDirection: Flickable.HorizontalFlick
                        acceptedButtons: Qt.NoButton
                        clip: true
                        ScrollBar.horizontal: AppScrollBar {}

                        Text {
                            id: tableText
                            textFormat: Text.RichText
                            color: root.textColor
                            font.family: Theme.font
                            font.pixelSize: root.baseSize
                            onLinkActivated: link => Util.openUrl(link)
                            text: {
                                const block = slot.block;
                                let html = `<table style="border-collapse: collapse; border-color: ${Theme.neutral50}" border="1" cellspacing="0" cellpadding="${Math.round(0.35 * root.baseSize)}">`;
                                for (const row of block.rows) {
                                    html += "<tr>";
                                    row.cells.forEach((cell, i) => {
                                        const tag = row.header ? "th" : "td";
                                        const align = row.header ? "center" : block.alignments[i];
                                        html += `<${tag} align="${align}">${cell}</${tag}>`;
                                    });
                                    html += "</tr>";
                                }
                                return root.richText(html + "</table>", 1.25, root.baseSize);
                            }
                        }
                    }
                }

                Component {
                    id: rule
                    Rectangle {
                        width: parent.width
                        height: 1
                        color: Theme.neutral80
                    }
                }
            }
        }
    }
}
