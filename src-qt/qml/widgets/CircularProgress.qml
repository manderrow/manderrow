import QtQuick
import QtQuick.Shapes
import Manderrow

// A circular progress ring, ported from CircularProgressIndicator.
Item {
    id: root

    /// `{ completed, total }`
    property var progress
    /// The font size the indicator is drawn at. It is 2em wide.
    property real size: Theme.rem
    property bool noProgressText: false

    readonly property real percentage: progress && progress.total ? progress.completed / progress.total * 100 : 0
    // the SVG this replaces was 200 units wide, with a ring of radius 90 and a stroke of 0.75em
    readonly property real _scale: width / 200
    readonly property real _stroke: 0.75 * size

    implicitWidth: 2 * size
    implicitHeight: implicitWidth

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            fillColor: "transparent"
            strokeColor: Theme.lighten(Theme.neutral80, -3)
            strokeWidth: root._stroke * root._scale

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: 90 * root._scale
                radiusY: radiusX
                startAngle: 0
                sweepAngle: 360
            }
        }

        ShapePath {
            fillColor: "transparent"
            strokeColor: Theme.primary60
            strokeWidth: root._stroke * root._scale
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: 90 * root._scale
                radiusY: radiusX
                startAngle: -90
                sweepAngle: 360 * root.percentage / 100

                Behavior on sweepAngle {
                    NumberAnimation {
                        duration: 100
                        easing.type: Easing.InOutQuad
                    }
                }
            }
        }
    }

    Text {
        visible: !root.noProgressText
        anchors.centerIn: parent
        text: Math.round(root.percentage) + "%"
        color: Theme.neutral20
        font.family: Theme.font
        font.pixelSize: 2.5 * root.size * root._scale
    }
}
