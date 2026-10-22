import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// CSS's `overflow-x: auto`: scrolls its content sideways when it is wider than this. The scroll
// bar is below the content rather than over it, and only takes up space while there is something
// to scroll. Vertical wheels pass through to whatever scrolls this up and down, as they do on the
// web, unless Shift is held, which turns them sideways. With nothing around it that scrolls up and
// down, they scroll it sideways without Shift.
Item {
    id: scroller

    default property alias content: flick.flickableData
    property real contentWidth
    property real contentHeight
    readonly property alias flickable: flick
    readonly property alias scrollBar: bar

    implicitHeight: flick.height + (bar.scrollable ? bar.height : 0)

    /// The nearest Flickable around this, which vertical wheels scroll when they can.
    readonly property Flickable _outer: {
        let item = parent;
        while (item && item.contentY === undefined)
            item = item.parent;
        return item;
    }
    /// Whether anything around this scrolls up and down.
    readonly property bool _outerScrolls: _outer !== null && _outer.visibleArea.heightRatio < 1

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
            acceptedModifiers: scroller._outerScrolls ? Qt.ShiftModifier : Qt.KeyboardModifierMask
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
