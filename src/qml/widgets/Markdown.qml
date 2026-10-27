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
    /// The source laid out as blocks, which this keeps as long as it shows them.
    readonly property MarkdownDocument document: App.markdown(source)
    /// Whether every block has been created.
    readonly property bool ready: blocks.ready
    /// How far its content reaches to the right: past its width where something can't wrap to fit
    /// in it, like a long word, which a view around it can scroll to, as browsers do.
    readonly property real overflowWidth: blocks.overflowWidth

    implicitHeight: blocks.height

    MarkdownBlocks {
        id: blocks
        width: parent.width
        blocks: root.document.blocks
        first: true
    }

    SelectionArea {
        content: blocks
    }
}
