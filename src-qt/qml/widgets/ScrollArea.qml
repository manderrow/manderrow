import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// CSS's `overflow: auto`, on both axes, for a Flickable declared inside it, which it sizes. Like
// a browser's classic scroll bars, its scroll bars take up space beside and below the Flickable,
// which cuts its content off before them, rather than being drawn over the content. They and the
// wheel scroll it from anywhere over the area. Horizontal wheels, and vertical ones with Shift
// held, scroll it sideways, as in browsers, as do vertical wheels without Shift while it only
// scrolls sideways.
//
// Leaving space for one scroll bar can make the other necessary, so the space is left once a bar
// appears or disappears, as in a browser's next layout. This settles, as the bars only ever make
// the Flickable smaller.
Item {
    id: area

    /// The Flickable to scroll, declared inside the area.
    required property Flickable flickable

    property real _verticalGutter: 0
    property real _horizontalGutter: 0
    Binding on _verticalGutter {
        value: verticalBar.gutter
        delayed: true
    }
    Binding on _horizontalGutter {
        value: horizontalBar.gutter
        delayed: true
    }

    Binding {
        target: area.flickable
        property: "width"
        value: area.width - area._verticalGutter
    }
    Binding {
        target: area.flickable
        property: "height"
        value: area.height - area._horizontalGutter
    }

    // attached to the Flickable, but kept here, outside it
    AppScrollBar {
        id: verticalBar
        z: 1
        anchors.right: parent.right
        height: area.flickable.height
    }
    AppScrollBar {
        id: horizontalBar
        z: 1
        orientation: Qt.Horizontal
        anchors.bottom: parent.bottom
        width: area.flickable.width
    }
    Component.onCompleted: {
        flickable.ScrollBar.vertical = verticalBar;
        flickable.ScrollBar.horizontal = horizontalBar;
    }

    WheelScroll {
        id: upDown
        flickable: area.flickable
        // leaves the wheel with Shift held to scroll only sideways, while there is anything to
        acceptedModifiers: sideways.enabled ? Qt.NoModifier : Qt.KeyboardModifierMask
    }
    WheelScroll {
        id: sideways
        flickable: area.flickable
        orientation: Qt.Horizontal
        // passes the wheel on while there is nothing to scroll sideways
        enabled: sideways.canScroll(0, false)
    }
    WheelHandler {
        target: null
        enabled: sideways.enabled
        orientation: Qt.Vertical
        acceptedModifiers: upDown.canScroll(0, true) ? Qt.ShiftModifier : Qt.KeyboardModifierMask
        acceptedDevices: sideways.acceptedDevices
        onWheel: event => sideways.scrollFor(event)
    }
}
