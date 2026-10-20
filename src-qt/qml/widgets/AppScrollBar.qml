import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A scroll bar styled like the web UI's (`scrollbar-color: neutral-40 transparent`). It is visible
// whenever there is something to scroll, unless `autoHide` is set, which shows it only while the
// pointer is over `hoverArea`, like overlayscrollbars with `autoHide: "leave"`.
ScrollBar {
    id: bar

    /// Narrower, for `scrollbar-width: thin`.
    property bool thin: false
    property bool autoHide: false
    property Item hoverArea: parent

    /// Whether there is anything to scroll.
    readonly property bool scrollable: size < 1.0
    /// Takes input at the window's edges over ResizeHandles, so a bar there can be dragged.
    readonly property bool claimsWindowEdge: scrollable

    padding: 2
    minimumSize: 0.05
    policy: scrollable ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff

    HoverHandler {
        id: areaHover
        parent: bar.hoverArea
    }

    contentItem: Rectangle {
        implicitWidth: bar.thin ? 6 : (bar.hovered || bar.pressed ? 10 : 8)
        implicitHeight: implicitWidth
        radius: width / 2
        color: bar.pressed ? Theme.neutral30 : bar.hovered ? Theme.neutral35 : Theme.neutral40
        opacity: !bar.scrollable ? 0 : !bar.autoHide || areaHover.hovered || bar.active ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 200
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.ease
            }
        }
        Behavior on implicitWidth {
            NumberAnimation {
                duration: 100
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.ease
            }
        }
    }

    background: null
}
