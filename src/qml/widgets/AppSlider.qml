import QtQuick
import QtQuick.Controls.Basic as Basic
import Manderrow

// A horizontal slider in the app's style: a thin track filled up to a round handle. The arrow keys
// move it by `stepSize` while it has the focus.
Basic.Slider {
    id: control

    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    implicitWidth: 10 * Theme.rem
    implicitHeight: handle.implicitHeight

    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        width: control.availableWidth
        height: 4
        radius: height / 2
        color: Theme.neutral70

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: control.enabled ? Theme.primary50 : Theme.neutral50
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + (control.availableHeight - height) / 2
        implicitWidth: Theme.rem
        implicitHeight: Theme.rem
        radius: width / 2
        color: control.pressed ? Theme.neutral20 : control.hovered ? Theme.neutral10 : Theme.neutral05
        border.color: Theme.primary50
        border.width: Ui.snapBorder(2)

        PixelSnap {}

        FocusRing {
            target: control
            radius: parent.radius
            anchors.margins: -3
        }
    }

    HoverHandler {
        cursorShape: control.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
    }
}
