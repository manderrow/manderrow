import QtQuick
import Manderrow

// A linear progress bar, ported from SimpleProgressIndicator. It is indeterminate while the total
// is unknown.
Item {
    id: root

    /// How much is done, of the `total`, as a task's progress.
    property real completed
    /// How much there is to do, or 0 while that's unknown.
    property real total
    property real fontSize: Theme.rem

    readonly property bool indeterminate: total === 0
    readonly property real fraction: indeterminate ? 0 : Math.min(1, completed / total)

    implicitWidth: 12 * Theme.rem
    implicitHeight: indeterminate ? 0.9 * fontSize : label.implicitHeight

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: Theme.lighten(Theme.neutral80, -3)
        clip: true

        Rectangle {
            visible: !root.indeterminate
            height: parent.height
            width: parent.width * root.fraction
            color: Theme.primary50

            Behavior on width {
                SmoothNumberAnimation {
                    duration: 200
                }
            }
        }

        Rectangle {
            id: runner

            property real position: 0

            visible: root.indeterminate
            height: parent.height
            width: parent.width / 2
            // CSS: left: t * 100%, translate: (2t - 1) * 100%
            x: -width + position * (parent.width + 2 * width)
            color: Theme.primary50
            transform: Scale {
                id: runnerScale
                origin.x: runner.width / 2
                xScale: 0.5
            }

            NumberAnimation on position {
                running: root.indeterminate && root.visible
                from: 0
                to: 1
                duration: 1500
                loops: Animation.Infinite
            }

            SequentialAnimation {
                running: root.indeterminate && root.visible
                loops: Animation.Infinite

                SmoothNumberAnimation {
                    target: runnerScale
                    property: "xScale"
                    from: 0.5
                    to: 0.8
                    duration: 600
                    curve: Theme.easeInOut
                }
                SmoothNumberAnimation {
                    target: runnerScale
                    property: "xScale"
                    to: 0.2
                    duration: 450
                    curve: Theme.easeInOut
                }
                SmoothNumberAnimation {
                    target: runnerScale
                    property: "xScale"
                    to: 0.15
                    duration: 300
                    curve: Theme.easeInOut
                }
                SmoothNumberAnimation {
                    target: runnerScale
                    property: "xScale"
                    to: 1
                    duration: 150
                    curve: Theme.easeInOut
                }
            }
        }
    }

    Text {
        id: label
        visible: !root.indeterminate
        anchors.centerIn: parent
        text: Util.formatRounded(root.fraction * 100) + "%"
        color: Theme.neutral05
        font.family: Theme.font
        font.pixelSize: 0.9 * Theme.rem
        style: Text.Raised
        styleColor: Qt.rgba(0, 0, 0, 0.3)
    }
}
