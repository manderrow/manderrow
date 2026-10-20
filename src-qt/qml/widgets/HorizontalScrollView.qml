import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// CSS's `overflow-x: auto`: scrolls its content sideways when it is wider than this. The scroll
// bar is below the content rather than over it, and only takes up space while there is something
// to scroll. Vertical wheels pass through to whatever scrolls this, as they do on the web.
Item {
    id: scroller

    default property alias content: flick.flickableData
    property real contentWidth
    property real contentHeight
    readonly property alias flickable: flick
    readonly property alias scrollBar: bar

    implicitHeight: flick.height + (bar.scrollable ? bar.height : 0)

    Flickable {
        id: flick
        width: parent.width
        height: scroller.contentHeight
        contentWidth: scroller.contentWidth
        contentHeight: scroller.contentHeight
        flickableDirection: Flickable.HorizontalFlick
        interactive: false
        clip: true
        ScrollBar.horizontal: bar
    }

    AppScrollBar {
        id: bar
        orientation: Qt.Horizontal
        thin: true
        y: flick.height
        width: parent.width
    }
}
