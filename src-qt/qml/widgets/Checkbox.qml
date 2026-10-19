import QtQuick
import Manderrow

// A checkbox whose check mark wipes in, ported from Checkbox.tsx.
Rectangle {
    id: box

    property bool checked: false
    property real size: Theme.rem

    signal toggled(bool checked)

    implicitWidth: size
    implicitHeight: size
    radius: 2
    color: checked ? Theme.primary40 : "transparent"
    border.color: Theme.neutral60
    border.width: 1

    Item {
        anchors.centerIn: parent
        width: icon.width * (box.checked ? 1 : 0)
        height: icon.height
        clip: true

        Behavior on width {
            NumberAnimation {
                duration: 200
            }
        }

        FaIcon {
            id: icon
            name: "check"
            size: box.size * 0.8
        }
    }

    TapHandler {
        onTapped: box.toggled(!box.checked)
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
