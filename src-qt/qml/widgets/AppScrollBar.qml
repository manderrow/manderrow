import QtQuick
import QtQuick.Controls.Basic
import Manderrow

// A thin overlay scroll bar that hides when idle, like overlayscrollbars with autoHide.
ScrollBar {
    id: bar

    padding: 2
    minimumSize: 0.05

    contentItem: Rectangle {
        implicitWidth: bar.hovered || bar.pressed ? 8 : 6
        implicitHeight: implicitWidth
        radius: width / 2
        color: Theme.neutral40
        opacity: bar.policy === ScrollBar.AlwaysOn || (bar.active && bar.size < 1.0) ? (bar.hovered || bar.pressed ? 0.9 : 0.6) : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 200
            }
        }
        Behavior on implicitWidth {
            NumberAnimation {
                duration: 100
            }
        }
    }

    background: null
}
