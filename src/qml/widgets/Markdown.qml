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

    /// Opens a link in the document. One to a heading, like `#installation`, scrolls the view
    /// around it to the heading, as browsers do. Other relative links are to files in the mod's
    /// package, which there is nothing to open.
    function openLink(url: string) {
        if (url.startsWith("#"))
            scrollToAnchor(url.slice(1));
        else if (/^[a-z][a-z0-9+.-]*:/i.test(url))
            Util.openUrl(url);
        else
            console.warn(`Not opening relative link: ${url}`);
    }

    /// Scrolls the view around the document to the heading with an anchor, if there is one.
    function scrollToAnchor(fragment: string) {
        let anchor = fragment;
        try {
            anchor = decodeURIComponent(fragment);
        } catch (e) {}
        // GitHub prefixes the ids of headings with this, which links sometimes include
        anchor = anchor.toLowerCase().replace(/^user-content-/, "");
        const find = item => {
            if (item.markdownAnchor === anchor) // qmllint disable missing-property
                return item;
            for (let i = 0; i < item.children.length; i++) {
                const found = find(item.children[i]);
                if (found)
                    return found;
            }
            return null;
        };
        // `#` and `#top` are the top of the document
        const target = anchor === "" || anchor === "top" ? blocks : find(blocks);
        const scroller = target && Ui.focusNavigator ? Ui.focusNavigator.scrollerOf(target) : null;
        if (scroller)
            scroller.scrollTo(target.mapToItem(scroller.flickable.contentItem, 0, 0).y, false);
    }

    MarkdownBlocks {
        id: blocks
        width: parent.width
        blocks: root.document.blocks
        first: true
        openLink: root.openLink
    }

    SelectionArea {
        content: blocks
        openLink: root.openLink
    }
}
