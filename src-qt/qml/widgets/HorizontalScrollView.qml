import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// CSS's `overflow-x: auto`: scrolls its content sideways when it is wider than this. The scroll
// bar is below the content rather than over it, and only takes up space while there is something
// to scroll. Vertical wheels pass through to whatever scrolls this, as they do on the web, unless
// Shift is held, which turns them sideways.
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

        WheelScroll {
            id: wheel
            orientation: Qt.Horizontal
            // passes the wheel on while there is nothing to scroll
            enabled: wheel.flickable !== null && wheel.canScroll(0, false)
        }
        WheelHandler {
            target: null
            enabled: wheel.enabled
            orientation: Qt.Vertical
            acceptedModifiers: Qt.ShiftModifier
            acceptedDevices: wheel.acceptedDevices
            onWheel: event => wheel.scrollFor(event)
        }
    }

    AppScrollBar {
        id: bar
        orientation: Qt.Horizontal
        thin: true
        y: flick.height
        width: parent.width
    }
}
