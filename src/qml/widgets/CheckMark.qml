pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Shapes
import Manderrow

// A check mark revealed from left to right, like the web UI's
// `clip-path: inset(0 100% 0 0)` to `inset(0)` transition. The icon itself doesn't move.
Item {
    id: mark

    property bool checked: false
    property real size: Theme.rem
    property color color: Theme.neutral05
    /// Hides the mark at once when unchecked instead of wiping it out, for
    /// `visibility: hidden` on unchecked.
    property bool hideWhenUnchecked: false
    /// Draws the check so that its strokes fill the mark's width and height, whichever it reaches
    /// first, centered in the other, instead of as an icon of `size` with Lucide's margins.
    property bool fill: false

    implicitWidth: icon.implicitWidth
    implicitHeight: icon.implicitHeight

    Item {
        width: parent.width * (mark.checked ? 1 : 0)
        height: parent.height
        clip: true
        // the software renderer draws shapes over their clip, so it is at least hidden once wiped
        // out
        visible: (mark.checked || !mark.hideWhenUnchecked) && width > 0

        Behavior on width {
            // always enabled, since toggling it with `checked` would race the width's binding
            SmoothNumberAnimation {
                duration: 200
            }
        }

        Icon {
            id: icon
            visible: !mark.fill
            name: "check"
            size: mark.size
            color: mark.color
        }

        Loader {
            active: mark.fill
            width: mark.width
            height: mark.height
            sourceComponent: fillCheck
        }
    }

    // Lucide's check, `M20 6 9 17l-5-5` stroked 2 wide with round caps and joins on its 24 unit
    // grid, whose strokes span x 3 to 21 and y 5 to 18
    Component {
        id: fillCheck

        Shape {
            id: shape

            readonly property real unit: Math.min(width / 18, height / 13)
            readonly property real originX: (width - 18 * unit) / 2 - 3 * unit
            readonly property real originY: (height - 13 * unit) / 2 - 5 * unit

            preferredRendererType: Shape.CurveRenderer
            // for the software renderer, which only smooths it when asked
            antialiasing: true

            ShapePath {
                fillColor: "transparent"
                strokeColor: mark.color
                strokeWidth: 2 * shape.unit
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin

                PathPolyline {
                    path: [Qt.point(shape.originX + 20 * shape.unit, shape.originY + 6 * shape.unit), Qt.point(shape.originX + 9 * shape.unit, shape.originY + 17 * shape.unit), Qt.point(shape.originX + 4 * shape.unit, shape.originY + 12 * shape.unit)]
                }
            }
        }
    }
}
