import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A scroll bar styled like the web UI's (`scrollbar-color: neutral-40 transparent`). It is visible
// whenever there is something to scroll, unless `autoHide` is set, which shows it only while the
// pointer is over `hoverArea`, like overlayscrollbars with `autoHide: "leave"`.
//
// Like a browser's classic scroll bars, which take up space beside the content, the content of a
// scrolling view should leave `gutter` free for it, or it is drawn over the content, as overlay
// scroll bars are.
ScrollBar {
    id: bar

    /// Narrower, for `scrollbar-width: thin`.
    property bool thin: false
    property bool autoHide: false
    property Item hoverArea: parent

    /// Whether there is anything to scroll. A view that hasn't been given a size yet has a size of
    /// 0, but nothing to scroll, and one whose content is less than half a pixel longer than it,
    /// as rounding leaves content that fits at fractional pixel ratios, has nothing to scroll to,
    /// as WheelScroll's `canScroll` has it. The bar is as long as the view.
    readonly property bool scrollable: size > 0 && (1 / size - 1) * (orientation === Qt.Vertical ? height : width) > 0.5
    /// The space beside the content that the bar takes up while there is something to scroll: its
    /// width, or its height for a horizontal bar, when hovered.
    readonly property real gutter: scrollable ? (thin ? 6 : 10) + 2 * padding : 0
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
            SmoothNumberAnimation {
                duration: 200
            }
        }
        Behavior on implicitWidth {
            SmoothNumberAnimation {
                duration: 100
            }
        }
    }

    background: null
}
