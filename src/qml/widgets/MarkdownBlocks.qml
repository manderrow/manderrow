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
    component Progress: QtObject {
        /// How many loaders are creating blocks.
        property int pending: 0
    }
    /// How many of the blocks are still being created, shared with the nested blocks.
    property Progress progress: Progress {}
    /// Whether every block has been created.
    readonly property bool ready: progress.pending === 0
    /// How far the blocks reach to the right, past their width where they can't wrap to fit.
    readonly property real overflowWidth: {
        let right = width;
        for (let i = 0; i < children.length; i++) {
            if (children[i].overflowWidth !== undefined)
                right = Math.max(right, children[i].overflowWidth);
        }
        return right;
    }

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

    /// Wraps inline HTML with the styles rich text supports. `imageMaxWidth` and
    /// `imageDecodeWidth` are as `maxWidth` and `decodeWidth` are for Util.webImagesInHtml.
    function richText(html, lineHeight, size, imageMaxWidth, imageDecodeWidth) {
        return `<style>a { color: ${Theme.primary60}; text-decoration: none; } code { font-family: "${Theme.fontMonospace}"; background-color: ${Theme.neutral90}; color: ${Theme.neutral05}; font-size: ${Math.round(0.9 * rem)}px; } del { text-decoration: line-through; }</style>` + `<div style="line-height: ${Math.round(lineHeight * size)}px">${Util.webImagesInHtml(html, Ui.dpr, imageMaxWidth, imageDecodeWidth)}</div>`;
    }

    // Creates its blocks asynchronously, as long documents would otherwise take frames to create
    // at once, and counts itself in `progress` while it does.
    component BlocksLoader: Loader {
        property bool _counted: false
        asynchronous: true
        onStatusChanged: {
            const loading = status === Loader.Loading;
            if (loading !== _counted) {
                root.progress.pending += loading ? 1 : -1;
                _counted = loading;
            }
        }
        Component.onDestruction: if (_counted && root.progress)
            root.progress.pending--
    }

    component RichBlock: TextEdit {
        property string html
        property real size: root.rem
        property real lineHeight: 1.35
        /// The width, in device pixels, that its images are decoded at, at most. It only grows, as
        /// setting the text again leaves their space empty while they load again.
        property real imageDecodeWidth: Util.imageDecodeWidth(width, Ui.dpr)
        readonly property real overflowWidth: Math.max(width, contentWidth)

        width: parent ? parent.width : 0
        onWidthChanged: imageDecodeWidth = Math.max(imageDecodeWidth, Util.imageDecodeWidth(width, Ui.dpr))
        textFormat: TextEdit.RichText
        // only between words, as on the web: a word too long for the line overflows it
        wrapMode: TextEdit.WordWrap
        // selected, its links opened, and its cursor set by Markdown's SelectionArea
        readOnly: true
        selectByMouse: false
        selectionColor: Theme.primary50
        color: root.textColor
        font.family: Theme.font
        font.pixelSize: size
        text: root.richText(html, lineHeight, size, undefined, imageDecodeWidth)
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
            readonly property real overflowWidth: loader.item && loader.item.overflowWidth !== undefined ? loader.x + loader.item.overflowWidth : width

            width: root.width
            height: topGap + (loader.item ? loader.item.height : 0) + margin.bottom

            BlocksLoader {
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
                        readonly property real overflowWidth: headingText.overflowWidth

                        width: parent.width
                        height: headingText.height + (slot.block.level === 1 ? Ui.hairline : 0)

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
                            height: Ui.hairline
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
                        // a badge keeps its height while it loads
                        readonly property real minHeight: Util.isBadge(slot.block.src) ? Util.badgeHeight : 0

                        width: parent.width
                        // its size is kept while it reloads at another width
                        height: img.aspect > 0 ? img.height : minHeight

                        Image {
                            id: img
                            // decoded no wider than it is shown
                            source: parent.width > 0 ? Util.webImage(slot.block.src, Ui.dpr, Util.imageDecodeWidth(parent.width, Ui.dpr)) : ""
                            asynchronous: true
                            fillMode: Image.PreserveAspectFit
                            readonly property real aspect: implicitHeight > 0 ? implicitWidth / implicitHeight : 0
                            // min-height scales an image without a width of its own up, keeping its
                            // aspect ratio
                            readonly property real naturalWidth: slot.block.width ? slot.block.width : Math.max(implicitWidth, parent.minHeight * aspect)
                            // img { max-width: 100% }
                            width: Math.min(naturalWidth, parent.width)
                            height: aspect > 0 ? Math.max(width / aspect, parent.minHeight) : 0
                            x: slot.block.centered ? (parent.width - width) / 2 : 0

                            // a link around the image
                            ClickArea {
                                enabled: !!slot.block.href
                                activatesOnSpace: false
                                onActivated: Util.openUrl(slot.block.href)
                            }
                        }
                    }
                }

                Component {
                    id: code
                    Rectangle {
                        width: parent.width
                        height: codeScroller.height
                        color: Theme.neutral90

                        HorizontalScrollView {
                            id: codeScroller
                            width: parent.width
                            // pre { padding: 0.5rem }, which scrolls with the code
                            contentWidth: codeText.width + 2 * 0.5 * root.rem
                            contentHeight: codeText.height + 2 * 0.5 * root.rem

                            TextEdit {
                                id: codeText
                                x: 0.5 * root.rem
                                y: 0.5 * root.rem
                                text: slot.block.text
                                textFormat: TextEdit.PlainText
                                readOnly: true
                                selectByMouse: false
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
                        readonly property real overflowWidth: quoteBlocks.item ? Math.max(width, quoteBlocks.x + quoteBlocks.item.overflowWidth) : width

                        width: parent.width
                        height: quoteBlocks.height + 2 * 0.5 * root.baseSize

                        Rectangle {
                            width: 4
                            height: parent.height
                            color: Theme.neutral40
                        }
                        BlocksLoader {
                            id: quoteBlocks
                            x: 4 + root.baseSize
                            y: 0.5 * root.baseSize
                            width: parent.width - x - root.baseSize
                            Component.onCompleted: setSource("MarkdownBlocks.qml", {
                                blocks: slot.block.children,
                                progress: root.progress,
                                compact: true,
                                textColor: Theme.neutral20
                            })
                        }
                    }
                }

                Component {
                    id: alert
                    Column {
                        readonly property real overflowWidth: Math.max(width, alertTitle.width, alertBlocks.item ? alertBlocks.item.overflowWidth : 0)

                        width: parent.width

                        Row {
                            id: alertTitle
                            spacing: 0.5 * root.rem
                            bottomPadding: 0.75 * root.rem

                            Icon {
                                // the nearest Lucide icons to the Octicons that GitHub shows
                                name: ({ note: "info", tip: "lightbulb", important: "message-square-warning", warning: "triangle-alert", caution: "octagon-alert" })[slot.block.kind] || ""
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
                        BlocksLoader {
                            id: alertBlocks
                            width: parent.width
                            Component.onCompleted: setSource("MarkdownBlocks.qml", {
                                blocks: slot.block.children,
                                progress: root.progress
                            })
                        }
                    }
                }

                Component {
                    id: list
                    Column {
                        readonly property real overflowWidth: {
                            let right = width;
                            for (let i = 0; i < children.length; i++) {
                                if (children[i].overflowWidth !== undefined)
                                    right = Math.max(right, children[i].overflowWidth);
                            }
                            return right;
                        }

                        width: parent.width

                        Repeater {
                            model: slot.block.items

                            delegate: Item {
                                id: listItem

                                required property var modelData
                                required property int index
                                readonly property real overflowWidth: itemBlocks.item ? itemBlocks.x + itemBlocks.item.overflowWidth : width

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

                                BlocksLoader {
                                    id: itemBlocks
                                    x: 1.5 * root.baseSize
                                    width: parent.width - x
                                    Component.onCompleted: setSource("MarkdownBlocks.qml", {
                                        blocks: listItem.modelData.children,
                                        progress: root.progress,
                                        compact: slot.block.tight
                                    })
                                }
                            }
                        }
                    }
                }

                Component {
                    id: table
                    // Qt's rich text sizes table columns itself, as browsers do, so the table is
                    // rendered as HTML. It is as wide as its content, or narrower with its cells
                    // wrapped, and it scrolls once they can't wrap any narrower.
                    HorizontalScrollView {
                        id: tableScroller
                        width: parent.width
                        contentWidth: Math.max(tableText.width, tableText.contentWidth)
                        contentHeight: tableText.height

                        TextEdit {
                            id: tableText
                            width: Math.min(implicitWidth, tableScroller.width)
                            wrapMode: TextEdit.WordWrap
                            textFormat: TextEdit.RichText
                            readOnly: true
                            selectByMouse: false
                            selectionColor: Theme.primary50
                            color: root.textColor
                            font.family: Theme.font
                            font.pixelSize: root.baseSize
                            text: {
                                const block = slot.block;
                                const padding = Math.round(0.35 * root.baseSize);
                                let html = `<table style="border-collapse: collapse; border-color: ${Theme.neutral50}" border="1" cellspacing="0" cellpadding="${padding}">`;
                                for (const row of block.rows) {
                                    html += "<tr>";
                                    row.cells.forEach((cell, i) => {
                                        const tag = row.header ? "th" : "td";
                                        const align = row.header ? "center" : block.alignments[i];
                                        html += `<${tag} align="${align}">${cell}</${tag}>`;
                                    });
                                    html += "</tr>";
                                }
                                // Rich text takes an image's percentage width of the whole table
                                // rather than of its cell, so each column's images get an equal
                                // share of the width instead, inside the table's border and the
                                // cells' padding and borders, as browsers shrink a row of images
                                // to fit.
                                const columns = Math.max(1, block.alignments.length);
                                const imageMaxWidth = (tableScroller.width - 1) / columns - 2 * padding - 2;
                                return root.richText(html + "</table>", 1.25, root.baseSize, imageMaxWidth, Util.imageDecodeWidth(imageMaxWidth, Ui.dpr));
                            }
                        }
                    }
                }

                Component {
                    id: rule
                    Rectangle {
                        width: parent.width
                        height: Ui.hairline
                        color: Theme.neutral80
                    }
                }
            }
        }
    }
}
