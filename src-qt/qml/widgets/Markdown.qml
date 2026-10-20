import QtQuick
import Manderrow

// Renders GitHub-flavoured Markdown, styled like the web UI's Markdown.css. Its text can be
// selected from one block to another, as on the web.
//
// Its blocks are created over several frames: show it once it is `ready`, or it is seen being
// laid out as they come in.
Item {
    id: root

    property string source
    /// Whether every block has been created.
    readonly property bool ready: blocks.ready

    implicitHeight: blocks.height

    MarkdownBlocks {
        id: blocks
        width: parent.width
        blocks: JSON.parse(Backend.markdownBlocks(root.source))
        first: true
    }

    SelectionArea {
        content: blocks
    }
}
